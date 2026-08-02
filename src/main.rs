#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! RuNote —— 现代、简单、轻量的 Windows 便签
//! 特性：
//!   - 卡片式便签列表 + 搜索过滤，悬停/选中带平滑动效
//!   - 自动保存（输入停顿 700ms 防抖 + 每 5 秒兜底 + 退出强制保存）
//!   - 内嵌开源中文字体（Noto Sans SC 子集，GB2312 全量），中文输入显示零依赖
//!   - 数据持久化到 %APPDATA%\RuNote\notes.json（原子写入）
//!   - 单个 exe 静态链接，运行时零外部依赖

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui;
use eframe::egui::{Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use serde::{Deserialize, Serialize};

// ---------------- 主题 ----------------
const ACCENT: Color32 = Color32::from_rgb(88, 122, 250);
const ERROR_RED: Color32 = Color32::from_rgb(232, 88, 88);
const TEXT_DARK: Color32 = Color32::from_rgb(52, 56, 68);
const TEXT_SOFT: Color32 = Color32::from_rgb(122, 128, 140);
const TEXT_FAINT: Color32 = Color32::from_rgb(170, 176, 188);
const BG_PANEL: Color32 = Color32::from_rgb(246, 247, 250);

// 便签卡片配色（柔和现代）
const NOTE_COLORS: [(u8, u8, u8); 6] = [
    (255, 243, 176), // 鹅黄
    (255, 219, 219), // 粉
    (216, 231, 255), // 蓝
    (213, 245, 220), // 绿
    (235, 223, 255), // 紫
    (255, 230, 199), // 杏橙
];

fn color_for(i: usize) -> Color32 {
    let (r, g, b) = NOTE_COLORS[i % NOTE_COLORS.len()];
    Color32::from_rgb(r, g, b)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        lerp(a.r() as f32, b.r() as f32, t) as u8,
        lerp(a.g() as f32, b.g() as f32, t) as u8,
        lerp(a.b() as f32, b.b() as f32, t) as u8,
        lerp(a.a() as f32, b.a() as f32, t) as u8,
    )
}

/// 截断字符串，超出部分以省略号代替
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_owned()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

// ---------------- 数据模型 ----------------

#[derive(Serialize, Deserialize, Clone)]
struct Note {
    id: u64,
    title: String,
    content: String,
    color: usize,
    created: u64, // unix 秒
    updated: u64,
}

impl Note {
    fn new(id: u64, color: usize) -> Self {
        let now = now_secs();
        Self { id, title: String::new(), content: String::new(), color, created: now, updated: now }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Notes {
    notes: Vec<Note>,
    next_id: u64,
}

// ---------------- 时间工具（UTC+8 显示） ----------------

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn ymd(secs_utc8: u64) -> (i64, u32, u32) {
    let z = (secs_utc8 / 86400) as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn fmt_time(secs: u64) -> String {
    let s = secs + 8 * 3600;
    let now = now_secs() + 8 * 3600;
    let (y, m, d) = ymd(s);
    let (ny, nm, nd) = ymd(now);
    let hh = (s % 86400) / 3600;
    let mm = (s % 3600) / 60;
    if (y, m, d) == (ny, nm, nd) {
        format!("{:02}:{:02}", hh, mm)
    } else if y == ny {
        format!("{:02}-{:02} {:02}:{:02}", m, d, hh, mm)
    } else {
        format!("{}-{:02}-{:02}", y, m, d)
    }
}

fn fmt_ago(secs: u64) -> String {
    let diff = now_secs().saturating_sub(secs);
    if diff < 60 {
        "刚刚".into()
    } else if diff < 3600 {
        format!("{} 分钟前", diff / 60)
    } else if diff < 86400 {
        format!("{} 小时前", diff / 3600)
    } else if diff < 172800 {
        "昨天".into()
    } else if diff < 86400 * 30 {
        format!("{} 天前", diff / 86400)
    } else {
        format!("{} 个月前", diff / (86400 * 30))
    }
}

// ---------------- 存储 ----------------

fn data_path() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("RuNote").join("notes.json")
    } else {
        PathBuf::from("notes.json")
    }
}

fn load_notes(path: &PathBuf) -> Notes {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// ---------------- 字体（内嵌 Noto Sans SC 子集） ----------------

const EMBEDDED_FONT: &[u8] = include_bytes!("../assets/font.otf");

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".to_owned(), egui::FontData::from_owned(EMBEDDED_FONT.to_vec()));
    // 把中文字体提到最前，中文英文统一用 Noto 渲染，杜绝缺字
    for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        if let Some(list) = fonts.families.get_mut(&fam) {
            list.insert(0, "cjk".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}

fn setup_text_styles(ctx: &egui::Context) {
    use egui::{FontFamily, TextStyle};
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(21.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(15.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(15.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
    ]
    .into();
    ctx.set_style(style);
}

fn setup_visuals(ctx: &egui::Context) {
    // 强制浅色主题，不跟随操作系统的深色/浅色模式设置，
    // 避免在系统开启深色模式时被自动切回黑色背景。
    ctx.set_theme(egui::Theme::Light);

    let mut v = egui::Visuals::light();
    v.panel_fill = BG_PANEL;
    v.window_fill = Color32::from_rgb(255, 255, 255);
    v.extreme_bg_color = Color32::from_rgb(255, 255, 255);
    v.faint_bg_color = Color32::from_rgb(244, 245, 249);
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.inactive.bg_fill = Color32::from_rgb(238, 240, 247);
    v.widgets.inactive.rounding = egui::Rounding::same(8.0);
    v.widgets.hovered.bg_fill = Color32::from_rgb(221, 228, 255);
    v.widgets.hovered.rounding = egui::Rounding::same(8.0);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.rounding = egui::Rounding::same(8.0);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(72, 78, 94));
    // 编辑区文字统一用深色，确保白底上清晰可读
    v.override_text_color = Some(TEXT_DARK);
    v.text_cursor.stroke = Stroke::new(2.0_f32, ACCENT);
    v.window_rounding = egui::Rounding::same(10.0);
    ctx.set_visuals(v);
}

// ---------------- 应用 ----------------

struct NoteApp {
    notes: Notes,
    current: Option<u64>,
    dirty: bool,
    dirty_since: Option<Instant>,
    last_save: Instant,
    save_path: PathBuf,
    search: String,
    confirm_delete: bool,
    status: String,
    status_ok: bool,
    logo_texture: Option<egui::TextureHandle>,
    /// 右键菜单目标便签 id（在便签列表卡片上右键弹出删除菜单）
    context_menu_note: Option<u64>,
}

impl NoteApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        setup_text_styles(&cc.egui_ctx);
        setup_visuals(&cc.egui_ctx);
        let save_path = data_path();
        let notes = load_notes(&save_path);

        // 侧栏 Logo 图标：直接用应用图标（与任务栏图标保持一致）
        let logo_texture = image::load_from_memory(APP_ICON).ok().map(|img| {
            let img = img.into_rgba8();
            let (w, h) = img.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
            cc.egui_ctx.load_texture("app_logo", color_image, egui::TextureOptions::LINEAR)
        });

        Self {
            notes,
            current: None,
            dirty: false,
            dirty_since: None,
            last_save: Instant::now(),
            save_path,
            search: String::new(),
            confirm_delete: false,
            status: "就绪，自动保存已开启".to_owned(),
            status_ok: true,
            logo_texture,
            context_menu_note: None,
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        if self.dirty_since.is_none() {
            self.dirty_since = Some(Instant::now());
        }
        if let Some(id) = self.current {
            if let Some(n) = self.notes.notes.iter_mut().find(|n| n.id == id) {
                n.updated = now_secs();
            }
        }
        self.status = "正在输入…".to_owned();
        self.status_ok = true;
    }

    fn save(&mut self) {
        if let Some(parent) = self.save_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(&self.notes).unwrap_or_default();
        let tmp = self.save_path.with_extension("json.tmp");
        match fs::write(&tmp, json).and_then(|_| fs::rename(&tmp, &self.save_path)) {
            Ok(()) => {
                self.status = "已自动保存".to_owned();
                self.status_ok = true;
            }
            Err(e) => {
                self.status = format!("保存失败: {e}");
                self.status_ok = false;
            }
        }
        self.dirty = false;
        self.dirty_since = None;
        self.last_save = Instant::now();
    }

    fn new_note(&mut self) {
        let id = self.notes.next_id;
        self.notes.next_id += 1;
        let color = (id as usize) % NOTE_COLORS.len();
        self.notes.notes.push(Note::new(id, color));
        self.current = Some(id);
        self.confirm_delete = false;
        self.mark_dirty();
        self.save();
    }

    fn delete_current(&mut self) {
        if let Some(id) = self.context_menu_note.or(self.current) {
            self.notes.notes.retain(|n| n.id != id);
            if self.current == Some(id) {
                self.current = None;
            }
            self.context_menu_note = None;
            self.confirm_delete = false;
            self.mark_dirty();
            self.save();
        }
    }

    // ---------- 侧栏 ----------

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);

        // Logo 行（与任务栏图标保持一致）
        ui.horizontal(|ui| {
            if let Some(tex) = &self.logo_texture {
                ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::new(18.0, 18.0)));
            } else {
                let (r, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::hover());
                ui.painter().circle_filled(r.center(), 8.0, ACCENT);
                ui.painter().circle_filled(r.center() - Vec2::new(1.5, 1.5), 4.5, Color32::WHITE);
            }
            ui.add_space(3.0);
            ui.label(RichText::new("RuNote").size(20.5).strong().color(TEXT_DARK));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("{} 张", self.notes.notes.len())).size(12.0).color(TEXT_FAINT));
            });
        });
        ui.add_space(12.0);

        // 新建按钮
        let new_btn = egui::Button::new(RichText::new("＋ 新建便签").color(Color32::WHITE).strong())
            .fill(ACCENT)
            .stroke(Stroke::NONE)
            .rounding(8.0)
            .min_size(Vec2::new(ui.available_width(), 34.0));
        let resp = ui.add(new_btn);
        if resp.clicked() {
            self.new_note();
        }
        if resp.hovered() {
            resp.on_hover_text("快捷键 Ctrl+N");
        }
        ui.add_space(8.0);

        // 搜索框
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("搜索便签…")
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::same(8.0)),
        );
        ui.add_space(8.0);

        // 卡片列表
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let q = self.search.trim().to_lowercase();
            let mut ids: Vec<u64> = self
                .notes
                .notes
                .iter()
                .filter(|n| {
                    q.is_empty()
                        || n.title.to_lowercase().contains(&q)
                        || n.content.to_lowercase().contains(&q)
                })
                .map(|n| n.id)
                .collect();
            ids.sort_by_key(|&id| {
                let n = self.notes.notes.iter().find(|x| x.id == id).unwrap();
                std::cmp::Reverse(n.updated)
            });

            if ids.is_empty() {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(if q.is_empty() { "还没有便签" } else { "没有匹配的便签" })
                            .size(14.0)
                            .color(TEXT_SOFT),
                    );
                });
            }

            let mut clicked: Option<u64> = None;
            for id in ids {
                let (title, preview, updated, color_idx, selected) = {
                    let n = self.notes.notes.iter().find(|x| x.id == id).unwrap();
                    let title = if n.title.trim().is_empty() {
                        "无标题".to_owned()
                    } else {
                        n.title.clone()
                    };
                    let preview = first_line(&n.content);
                    (title, preview, n.updated, n.color, self.current == Some(id))
                };

                // 卡片主体：手绘 + 平滑动效
                let height = 76.0;
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::click());
                let hovered = resp.hovered() || resp.dragged();
                let t = ui
                    .ctx()
                    .animate_bool_with_time(egui::Id::new(("card", id)), hovered || selected, 0.16);

                let base = color_for(color_idx);
                let base_dim = base.gamma_multiply(0.86);
                let fill = lerp_color(base_dim, base, t);
                let border = if selected {
                    Stroke::new(1.6_f32, ACCENT)
                } else {
                    Stroke::new(1.0_f32, Color32::from_black_alpha(12))
                };
                let painter = ui.painter_at(rect);
                // 柔和阴影（hover 时加深、轻微上浮）
                let shadow_alpha = lerp(10.0, 26.0, t) as u8;
                let lift = lerp(1.5, 3.0, t);
                painter.rect_filled(
                    rect.translate(Vec2::new(0.0, lift)),
                    egui::Rounding::same(10.0),
                    Color32::from_black_alpha(shadow_alpha),
                );
                painter.rect_filled(rect, egui::Rounding::same(10.0), fill);
                painter.rect_stroke(rect, egui::Rounding::same(10.0), border);

                // 选中指示条
                if selected {
                    painter.rect_filled(
                        Rect::from_min_max(
                            Pos2::new(rect.left() + 5.0, rect.top() + 14.0),
                            Pos2::new(rect.left() + 8.0, rect.bottom() - 14.0),
                        ),
                        egui::Rounding::same(1.5),
                        ACCENT,
                    );
                }

                // 文字
                let x = rect.left() + 14.0;
                let w = rect.width() - 30.0;
                let title_galley = painter.layout(
                    truncate(&title, 20),
                    FontId::proportional(14.5),
                    TEXT_DARK,
                    w,
                );
                painter.galley(Pos2::new(x, rect.top() + 9.0), title_galley, TEXT_DARK);

                if !preview.is_empty() {
                    let g = painter.layout(
                        truncate(&preview, 34),
                        FontId::proportional(12.5),
                        TEXT_SOFT,
                        w,
                    );
                    painter.galley(Pos2::new(x, rect.top() + 30.0), g, TEXT_SOFT);
                }

                let tg = painter.layout(fmt_time(updated), FontId::proportional(11.0), TEXT_FAINT, 80.0);
                painter.galley(
                    Pos2::new(rect.right() - tg.size().x - 10.0, rect.bottom() - tg.size().y - 6.0),
                    tg,
                    TEXT_FAINT,
                );

                if resp.clicked() {
                    clicked = Some(id);
                }

                // 右键弹出删除菜单
                let mut delete_requested = false;
                resp.context_menu(|ui| {
                    ui.set_min_width(96.0);
                    let del_btn = egui::Button::new(RichText::new("删除").color(ERROR_RED));
                    if ui.add(del_btn).clicked() {
                        delete_requested = true;
                        ui.close_menu();
                    }
                });
                if delete_requested {
                    self.context_menu_note = Some(id);
                    self.confirm_delete = true;
                }

                ui.add_space(6.0);
            }
            if let Some(id) = clicked {
                self.current = Some(id);
                self.confirm_delete = false;
            }
        });
    }

    // ---------- 编辑区 ----------

    fn editor(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.current else {
            self.empty_state(ui);
            return;
        };
        let Some(idx) = self.notes.notes.iter().position(|n| n.id == id) else {
            self.current = None;
            return;
        };

        let (title_changed, content_changed, _created, updated, chars) = {
            let note = &mut self.notes.notes[idx];

            ui.add_space(10.0);
            // 标题
            let t_resp = ui.add(
                egui::TextEdit::singleline(&mut note.title)
                    .font(egui::TextStyle::Heading)
                    .text_color(TEXT_DARK)
                    .hint_text("无标题")
                    .desired_width(f32::INFINITY)
                    .frame(false),
            );
            let title_changed = t_resp.changed();
            ui.add_space(2.0);

            // 时间行（删除按钮已移至左侧便签列表：右键点击便签弹出）
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("创建于 {}", fmt_time(note.created))).size(12.0).color(TEXT_FAINT));
                ui.label(RichText::new("·").size(12.0).color(TEXT_FAINT));
                ui.label(RichText::new(fmt_ago(note.updated)).size(12.0).color(TEXT_FAINT));
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            // 正文：按剩余高度填满整个编辑区
            let row_h = ui.text_style_height(&egui::TextStyle::Body);
            let avail_h = ui.available_height() - 10.0;
            let rows = (avail_h / row_h).floor().max(8.0) as usize;
            let content_id = egui::Id::new(("note_content_editor", id));

            let c_resp = ui.add(
                egui::TextEdit::multiline(&mut note.content)
                    .id(content_id)
                    .desired_rows(rows)
                    .desired_width(f32::INFINITY)
                    .text_color(TEXT_DARK)
                    .frame(false)
                    .hint_text("写点什么…"),
            );

            // 持续记录"最近一次非空选区"，存在独立的 memory 键里（不是 TextEdit 自身的状态）。
            // egui 的 TextEdit 收到右键点击时会把它当成普通点击处理并清空选区，
            // 所以不能依赖 TextEdit 自身状态在右键那一帧的值，而是用这份独立记录的、
            // 点击发生前最后一次真实存在的选区来判断。
            let last_sel_key = egui::Id::new(("note_content_last_selection", id));
            let current_range =
                egui::widgets::text_edit::TextEditState::load(ui.ctx(), content_id).and_then(|s| s.cursor.char_range());
            if let Some(range) = current_range {
                if range.primary.index != range.secondary.index {
                    ui.ctx().data_mut(|d| d.insert_temp(last_sel_key, range));
                }
            }

            // 右键点击会让 TextEdit 把选区清空，导致蓝色高亮消失。
            // 如果检测到这一帧右键点击导致选区被清空了，就用刚才记录的快照把
            // TextEdit 自身的选区状态也恢复回去，这样界面上会重新显示选中高亮。
            if c_resp.secondary_clicked() {
                let now_empty = current_range.map_or(true, |r| r.primary.index == r.secondary.index);
                if now_empty {
                    let saved: Option<egui::text::CCursorRange> = ui.ctx().data(|d| d.get_temp(last_sel_key));
                    if let Some(saved_range) = saved {
                        if let Some(mut state) = egui::widgets::text_edit::TextEditState::load(ui.ctx(), content_id) {
                            state.cursor.set_char_range(Some(saved_range));
                            state.store(ui.ctx(), content_id);
                            ui.ctx().request_repaint();
                        }
                    }
                }
            }

            let content_changed = editor_context_menu(ui, &c_resp, content_id, last_sel_key, &mut note.content) || c_resp.changed();
            let chars = note.content.chars().count();
            (title_changed, content_changed, note.created, note.updated, chars)
        };

        if title_changed || content_changed {
            self.mark_dirty();
        }

        // 底部：字数
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{chars} 字")).size(11.5).color(TEXT_FAINT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("最近编辑 {updated_ago}", updated_ago = fmt_ago(updated)))
                    .size(11.5)
                    .color(TEXT_FAINT));
            });
        });
    }

    fn empty_state(&mut self, ui: &mut egui::Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                // 手绘便签图标
                let (rect, _) = ui.allocate_exact_size(Vec2::new(96.0, 96.0), egui::Sense::hover());
                let p = ui.painter_at(rect);
                let card = Rect::from_min_size(Pos2::new(rect.left() + 12.0, rect.top() + 10.0), Vec2::new(72.0, 76.0));
                p.rect_filled(card.translate(Vec2::new(0.0, 5.0)), egui::Rounding::same(16.0), Color32::from_black_alpha(12));
                p.rect_filled(card, egui::Rounding::same(16.0), Color32::from_rgb(255, 242, 199));
                // 顶部封条
                p.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(card.left() + 14.0, card.top() - 5.0),
                        Pos2::new(card.left() + 34.0, card.top() + 4.0),
                    ),
                    egui::Rounding::same(3.0),
                    ACCENT,
                );
                // 文字行
                let lc = Color32::from_rgb(192, 178, 128);
                for i in 0..4 {
                    let y = card.top() + 18.0 + i as f32 * 12.5;
                    let w = 44.0 - (i % 2) as f32 * 8.0;
                    p.rect_filled(
                        Rect::from_min_size(Pos2::new(card.left() + 12.0, y), Vec2::new(w, 3.5)),
                        egui::Rounding::same(1.75),
                        lc,
                    );
                }

                ui.add_space(12.0);
                ui.label(RichText::new("选择或新建一张便签").size(16.0).color(TEXT_SOFT));
                ui.add_space(4.0);
                ui.label(RichText::new("Ctrl+N 新建　·　Ctrl+S 保存").size(12.0).color(TEXT_FAINT));
            });
        });
    }
}

/// 为多行文本编辑器提供右键菜单：复制 / 剪切 / 粘贴 / 全选
/// `last_sel_key` 是外部持续维护的"最近一次非空选区"memory 键——
/// 因为 egui 的 TextEdit 收到右键点击时会把它当成普通点击处理并清空选区，
/// 所以复制/剪切是否可用、要复制剪切的内容范围，都以这份记录为准，
/// 而不是读右键点击那一帧 TextEdit 自身的（已被清空的）状态。
/// 返回 true 表示文本内容被本函数修改（剪切/粘贴），调用方需要据此标记为已改动。
fn editor_context_menu(
    ui: &mut egui::Ui,
    resp: &egui::Response,
    editor_id: egui::Id,
    last_sel_key: egui::Id,
    text: &mut String,
) -> bool {
    use egui::text::{CCursor, CCursorRange};
    use egui::widgets::text_edit::TextEditState;

    let mut changed = false;
    resp.context_menu(|ui| {
        ui.set_min_width(112.0);

        // 点击前最后一次真实存在的选区（字符索引），用于复制/剪切时提取被选中的文本
        let selected_range: Option<CCursorRange> = ui.ctx().data(|d| d.get_temp(last_sel_key));

        let selected_text: Option<String> = selected_range.and_then(|range| {
            let (min, max) = (
                range.primary.index.min(range.secondary.index),
                range.primary.index.max(range.secondary.index),
            );
            if min == max {
                None
            } else {
                Some(text.chars().skip(min).take(max - min).collect())
            }
        });
        let has_selection = selected_text.as_ref().map_or(false, |s| !s.is_empty());

        if ui.add_enabled(has_selection, egui::Button::new("复制")).clicked() {
            if let Some(s) = selected_text.clone() {
                ui.output_mut(|o| o.copied_text = s);
            }
            ui.close_menu();
        }

        if ui.add_enabled(has_selection, egui::Button::new("剪切")).clicked() {
            if let (Some(s), Some(range)) = (selected_text.clone(), selected_range) {
                ui.output_mut(|o| o.copied_text = s);
                let (min, max) = (
                    range.primary.index.min(range.secondary.index),
                    range.primary.index.max(range.secondary.index),
                );
                let before: String = text.chars().take(min).collect();
                let after: String = text.chars().skip(max).collect();
                *text = before + &after;
                changed = true;
                // 光标落到剪切位置，并清掉记录的选区，避免再次误用
                let mut state = TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
                state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(min))));
                state.store(ui.ctx(), editor_id);
                ui.ctx().data_mut(|d| d.remove::<CCursorRange>(last_sel_key));
            }
            ui.close_menu();
        }

        if ui.button("粘贴").clicked() {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                if let Ok(clip_text) = clipboard.get_text() {
                    if !clip_text.is_empty() {
                        // 粘贴优先用记录的选区（如果有），否则退回当前光标位置
                        let range = selected_range
                            .or_else(|| TextEditState::load(ui.ctx(), editor_id).and_then(|s| s.cursor.char_range()))
                            .unwrap_or_else(|| CCursorRange::one(CCursor::new(text.chars().count())));
                        let (min, max) = (
                            range.primary.index.min(range.secondary.index),
                            range.primary.index.max(range.secondary.index),
                        );
                        let before: String = text.chars().take(min).collect();
                        let after: String = text.chars().skip(max).collect();
                        let insert_chars = clip_text.chars().count();
                        *text = before + &clip_text + &after;
                        changed = true;
                        let new_pos = CCursor::new(min + insert_chars);
                        let mut state = TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
                        state.cursor.set_char_range(Some(CCursorRange::one(new_pos)));
                        state.store(ui.ctx(), editor_id);
                        ui.ctx().data_mut(|d| d.remove::<CCursorRange>(last_sel_key));
                    }
                }
            }
            ui.close_menu();
        }

        ui.separator();

        if ui.button("全选").clicked() {
            let start = CCursor::new(0);
            let end = CCursor::new(text.chars().count());
            let mut state = TextEditState::load(ui.ctx(), editor_id).unwrap_or_default();
            state.cursor.set_char_range(Some(CCursorRange::two(start, end)));
            state.store(ui.ctx(), editor_id);
            ui.ctx().data_mut(|d| d.insert_temp(last_sel_key, CCursorRange::two(start, end)));
            ui.close_menu();
        }
    });
    changed
}

/// 取内容第一行作为列表摘要
fn first_line(s: &str) -> String {
    let t = s.trim();
    match t.find('\n') {
        Some(i) => t[..i].trim().to_owned(),
        None => t.to_owned(),
    }
}

impl eframe::App for NoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ---- 自动保存：700ms 防抖 / 5s 兜底 ----
        if self.dirty {
            let debounced = self
                .dirty_since
                .map_or(false, |t| t.elapsed() >= Duration::from_millis(700));
            let heartbeat = self.last_save.elapsed() >= Duration::from_secs(5);
            if debounced || heartbeat {
                self.save();
            }
        }

        // ---- 快捷键 ----
        if ctx.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.command) {
            self.new_note();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command) {
            self.save();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.confirm_delete = false;
            self.context_menu_note = None;
        }

        // 底部状态栏已按需求隐藏（保存状态仍在内部记录，仅不再显示）

        // ---- 左侧列表（固定宽度，不可拖动） ----
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .default_width(286.0)
            .show(ctx, |ui| self.sidebar(ui));

        // ---- 中央编辑区（显式白底黑字，避免透出黑色背景） ----
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(255, 255, 255))
                    .inner_margin(egui::Margin::same(14.0)),
            )
            .show(ctx, |ui| self.editor(ui));

        // ---- 删除确认弹窗 ----
        if self.confirm_delete {
            let mut open = true;
            let mut decision: Option<bool> = None;
            egui::Window::new("删除便签")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.add_space(2.0);
                    ui.label("确定删除这张便签吗？此操作无法撤销。");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let del = egui::Button::new(RichText::new("删除").color(Color32::WHITE))
                                .fill(ERROR_RED)
                                .stroke(Stroke::NONE)
                                .rounding(6.0);
                            if ui.add(del).clicked() {
                                decision = Some(true);
                            }
                            if ui.button("取消").clicked() {
                                decision = Some(false);
                            }
                        });
                    });
                });
            match decision {
                Some(true) => self.delete_current(),
                Some(false) => {
                    self.confirm_delete = false;
                    self.context_menu_note = None;
                }
                None => {
                    if !open {
                        self.confirm_delete = false;
                        self.context_menu_note = None;
                    }
                }
            }
        }

        // 让相对时间文本持续刷新
        ctx.request_repaint_after(Duration::from_millis(300));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save();
    }
}

// ---------------- 入口 ----------------

/// 应用图标（内嵌，用于标题栏/任务栏，与 assets/app.ico 保持一致）
const APP_ICON: &[u8] = include_bytes!("../assets/app.ico");

fn load_app_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(APP_ICON).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData { rgba: image.into_raw(), width, height })
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("RuNote 便签")
        .with_inner_size([1180.0, 660.0])
        .with_min_inner_size([560.0, 400.0]);
    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        // 记住窗口位置/大小，下次启动恢复到上一次关闭时的状态
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native(
        "RuNote",
        options,
        Box::new(|cc| Ok(Box::new(NoteApp::new(cc)))),
    )
}
