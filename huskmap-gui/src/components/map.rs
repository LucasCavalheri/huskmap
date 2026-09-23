//! The Husk Map. A sonar dial over the disk: angle is kind, radius is age, size is weight,
//! color is what you are allowed to do. The beam lights husks as it passes.

use std::cell::Cell;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::rc::Rc;

use freya::animation::*;
use freya::engine::prelude::{
    Canvas as SkCanvas, Paint, PaintStyle, PathBuilder, SkBlurStyle, SkColor, SkMaskFilter, SkRect,
};
use freya::prelude::*;
use huskmap_core::{HuskId, HuskKind, copy, format_bytes};

use crate::components::ui::{caps, glyph, mono};
use crate::icons::Glyph;
use crate::theme::{self, Rgb};
use crate::view_model::{AppState, INNER_R, MapNode, OUTER_R, Tone, polar, rings, sectors};

/// Everything the dial needs, as plain data so it only re-renders when it changes.
#[derive(Debug, Clone, PartialEq)]
pub struct SectorLabel {
    pub kind: HuskKind,
    pub label: String,
    pub bytes: String,
    pub count: usize,
    pub active: bool,
}

#[derive(PartialEq)]
pub struct HuskMap {
    pub state: State<AppState>,
    pub nodes: Vec<MapNode>,
    pub sectors: Vec<SectorLabel>,
    pub generation: u64,
    pub scanning: bool,
}

fn sk(c: Rgb, a: f32) -> SkColor {
    SkColor::from_argb((a.clamp(0.0, 1.0) * 255.0) as u8, c.0, c.1, c.2)
}

fn paint(c: Rgb, a: f32, style: PaintStyle, width: f32) -> Paint {
    let mut p = Paint::default();
    p.set_anti_alias(true);
    p.set_style(style);
    p.set_stroke_width(width);
    p.set_color(sk(c, a));
    p
}

fn fill(c: Rgb, a: f32) -> Paint {
    paint(c, a, PaintStyle::Fill, 0.0)
}

fn stroke(c: Rgb, a: f32, w: f32) -> Paint {
    paint(c, a, PaintStyle::Stroke, w)
}

fn glow(c: Rgb, a: f32, sigma: f32) -> Paint {
    let mut p = fill(c, a);
    p.set_mask_filter(SkMaskFilter::blur(SkBlurStyle::Normal, sigma, false));
    p
}

/// Geometry of the dial inside a canvas of `w`×`h`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dial {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
}

impl Dial {
    pub fn fit(w: f32, h: f32) -> Self {
        Self {
            cx: w / 2.0,
            cy: h / 2.0 + 6.0,
            r: (w.min(h) * 0.39).max(40.0),
        }
    }

    pub fn at(&self, angle: f32, radius: f32) -> (f32, f32) {
        let (x, y) = polar(angle, radius);
        (self.cx + x * self.r, self.cy + y * self.r)
    }

    /// Pixel → unit dial coordinates.
    pub fn unit(&self, x: f32, y: f32) -> (f32, f32) {
        ((x - self.cx) / self.r, (y - self.cy) / self.r)
    }
}

pub fn node_px(weight: f32) -> f32 {
    3.2 + weight * 17.0
}

struct Frame<'a> {
    dial: Dial,
    sweep: f32,
    reveal: f32,
    beat: f32,
    scanning: bool,
    nodes: &'a [MapNode],
    sectors: &'a [SectorLabel],
    hover: Option<&'a HuskId>,
}

fn draw(canvas: &SkCanvas, w: f32, h: f32, f: &Frame<'_>) {
    let Dial { cx, cy, r } = f.dial;
    canvas.draw_rect(SkRect::from_wh(w, h), &fill(theme::pitch(), 1.0));

    // Soil: a warm bloom under the dial.
    canvas.draw_circle(
        (cx, cy),
        r * 1.05,
        &glow(theme::copper_deep(), 0.10, r * 0.35),
    );
    canvas.draw_circle((cx, cy), r * 0.55, &glow(theme::copper(), 0.035, r * 0.25));
    canvas.draw_circle((cx, cy), r * OUTER_R, &fill(theme::carbon(), 0.55));

    // Sector wedges.
    let secs = sectors();
    for (sec, label) in secs.iter().zip(f.sectors) {
        let alpha = if label.active { 0.035 } else { 0.0 };
        let mut wedge = PathBuilder::new();
        let inner = r * INNER_R * 0.8;
        let outer = r * OUTER_R;
        let a0 = sec.start + 0.012;
        let a1 = sec.start + sec.sweep - 0.012;
        let steps = 24;
        for i in 0..=steps {
            let a = a0 + (a1 - a0) * i as f32 / steps as f32;
            let (x, y) = f.dial.at(a, OUTER_R);
            if i == 0 {
                wedge.move_to((x, y));
            } else {
                wedge.line_to((x, y));
            }
        }
        for i in (0..=steps).rev() {
            let a = a0 + (a1 - a0) * i as f32 / steps as f32;
            let (x, y) = f.dial.at(a, inner / r);
            wedge.line_to((x, y));
        }
        wedge.close();
        canvas.draw_path(&wedge.detach(), &fill(theme::kind_color(sec.kind), alpha));
        let _ = outer;
        // divider
        let (x0, y0) = f.dial.at(sec.start, INNER_R * 0.8);
        let (x1, y1) = f.dial.at(sec.start, OUTER_R + 0.035);
        canvas.draw_line((x0, y0), (x1, y1), &stroke(theme::hairline(), 0.9, 1.0));
    }

    // Age rings, dotted.
    for (i, (_, rr)) in rings().iter().enumerate() {
        let dots = (rr * 220.0) as usize;
        let dot = fill(theme::ash(), 0.10 + i as f32 * 0.02);
        for d in 0..dots {
            let a = d as f32 / dots as f32 * TAU;
            let (x, y) = f.dial.at(a, *rr);
            canvas.draw_circle((x, y), 0.7, &dot);
        }
    }
    canvas.draw_circle((cx, cy), r * OUTER_R, &stroke(theme::hairline(), 1.0, 1.0));

    // Sonar beam with a fading tail.
    let beam = f.sweep;
    let tail = if f.scanning { 1.1 } else { 0.7 };
    let slices = 36;
    for i in 0..slices {
        let t0 = i as f32 / slices as f32;
        let t1 = (i + 1) as f32 / slices as f32;
        let a0 = beam - tail * t1;
        let a1 = beam - tail * t0;
        let mut wedge = PathBuilder::new();
        wedge.move_to((cx, cy));
        let (x0, y0) = f.dial.at(a0, OUTER_R);
        let (x1, y1) = f.dial.at(a1, OUTER_R);
        wedge.line_to((x0, y0));
        wedge.line_to((x1, y1));
        wedge.close();
        let strength = (1.0 - t0).powf(2.2) * if f.scanning { 0.16 } else { 0.07 };
        canvas.draw_path(&wedge.detach(), &fill(theme::amber(), strength));
    }
    let (bx, by) = f.dial.at(beam, OUTER_R);
    canvas.draw_line((cx, cy), (bx, by), &stroke(theme::amber(), 0.55, 1.2));
    canvas.draw_circle((bx, by), 2.2, &fill(theme::amber(), 0.9));
    canvas.draw_circle((bx, by), 7.0, &glow(theme::amber(), 0.5, 5.0));

    // Husks, heaviest first so small ones stay visible on top.
    let total = f.nodes.len().max(1) as f32;
    for node in f.nodes.iter() {
        let appear = node.rank as f32 / total * 0.65;
        let local = ((f.reveal - appear) / 0.35).clamp(0.0, 1.0);
        if local <= 0.0 {
            continue;
        }
        let ease = 1.0 - (1.0 - local).powi(3);
        let (x, y) = f.dial.at(node.angle, node.radius * (0.6 + 0.4 * ease));
        let size = node_px(node.weight) * (0.4 + 0.6 * ease);
        let behind = (beam - node.angle).rem_euclid(TAU);
        let lit = (1.0 - behind / 1.4).clamp(0.0, 1.0).powi(2);
        let color = theme::tone_color(node.tone);
        let hovered = f.hover == Some(&node.id);
        let base = if node.selected || hovered {
            1.0
        } else {
            0.62 + lit * 0.38
        };

        if lit > 0.02 || node.selected {
            canvas.draw_circle(
                (x, y),
                size * 1.9,
                &glow(
                    color,
                    (0.30 * lit + if node.selected { 0.35 } else { 0.0 }) * ease,
                    size * 0.9,
                ),
            );
        }
        if node.occupied {
            let p = f.beat;
            canvas.draw_circle(
                (x, y),
                size * (1.3 + p * 1.8),
                &stroke(theme::oxblood(), (1.0 - p) * 0.8 * ease, 1.4),
            );
        }

        // A sonar contact: a dim disc, a crisp rim, a bright core. Heavy ones get an inner
        // ring, like a return that echoes.
        let r = size * 0.78;
        canvas.draw_circle(
            (x, y),
            r,
            &fill(theme::mix(theme::pitch(), color, 0.28), ease),
        );
        canvas.draw_circle((x, y), r, &stroke(color, base * ease, 1.2));
        if r > 7.0 {
            canvas.draw_circle((x, y), r * 0.62, &stroke(color, 0.32 * base * ease, 0.8));
        }
        canvas.draw_circle(
            (x, y),
            (r * 0.26).clamp(1.3, 3.2),
            &fill(theme::mix(color, theme::bone(), 0.25), base * ease),
        );

        if node.marked {
            canvas.draw_circle(
                (x, y),
                size + 5.0,
                &stroke(theme::copper(), 0.95 * ease, 1.6),
            );
            canvas.draw_circle(
                (x + size * 0.8, y - size * 0.8),
                2.4,
                &fill(theme::copper(), ease),
            );
        }
        if node.selected || hovered {
            let ring = stroke(theme::bone(), if node.selected { 0.9 } else { 0.5 }, 1.0);
            canvas.draw_circle((x, y), size + 9.0, &ring);
            for a in [0.0f32, FRAC_PI_2, FRAC_PI_2 * 2.0, FRAC_PI_2 * 3.0] {
                let (dx, dy) = (a.cos(), a.sin());
                canvas.draw_line(
                    (x + dx * (size + 11.0), y + dy * (size + 11.0)),
                    (x + dx * (size + 17.0), y + dy * (size + 17.0)),
                    &ring,
                );
            }
        }
    }

    // The machine at the core.
    canvas.draw_circle((cx, cy), r * INNER_R * 0.55, &fill(theme::pitch(), 1.0));
    canvas.draw_circle(
        (cx, cy),
        r * INNER_R * 0.55,
        &stroke(theme::hairline(), 1.0, 1.0),
    );
    canvas.draw_circle((cx, cy), 4.0, &fill(theme::copper(), 1.0));
    canvas.draw_circle((cx, cy), 10.0, &glow(theme::copper(), 0.5, 6.0));
    canvas.draw_circle(
        (cx, cy),
        r * INNER_R * (0.55 + f.beat * 0.25),
        &stroke(theme::copper(), (1.0 - f.beat) * 0.35, 1.0),
    );
}

fn label_anchor(dial: &Dial, angle: f32, radius: f32, w: f32) -> (f32, f32, bool) {
    let (x, y) = dial.at(angle, radius);
    let right = angle.rem_euclid(TAU) < std::f32::consts::PI;
    (if right { x } else { x - w }, y, right)
}

impl Component for HuskMap {
    fn render(&self) -> impl IntoElement {
        let scanning = self.scanning;
        let sweep = use_animation_with_dependencies(&scanning, move |conf, scanning| {
            conf.on_creation(OnCreation::Run);
            conf.on_finish(OnFinish::restart());
            AnimNum::new(0.0, TAU)
                .time(if *scanning {
                    theme::SWEEP_SCAN
                } else {
                    theme::SWEEP_IDLE
                })
                .function(Function::Linear)
        });
        let beat = use_animation(|conf| {
            conf.on_creation(OnCreation::Run);
            conf.on_finish(OnFinish::restart());
            AnimNum::new(0.0, 1.0)
                .time(theme::HEARTBEAT)
                .function(Function::Quad)
                .ease(Ease::Out)
        });
        let reveal = use_animation_with_dependencies(&self.generation, |conf, _| {
            conf.on_creation(OnCreation::Run);
            AnimNum::new(0.0, 1.0)
                .time(theme::MOTION_REVEAL)
                .function(Function::Cubic)
                .ease(Ease::Out)
        });
        let mut hover = use_state(|| None::<HuskId>);
        let mut size = use_state(|| (900.0f32, 700.0f32));
        let measured = use_hook(|| Rc::new(Cell::new((900.0f32, 700.0f32))));
        // RenderCallback always compares equal, so a fresh key is what makes Skia repaint.
        let frame = use_hook(|| Rc::new(Cell::new(0u64)));
        frame.set(frame.get().wrapping_add(1));

        let (w, h) = *size.read();
        let dial = Dial::fit(w, h);
        let sweep_v = sweep.get().value();
        let beat_v = beat.get().value();
        let reveal_v = if self.scanning {
            1.0
        } else {
            reveal.get().value()
        };

        let nodes = self.nodes.clone();
        let sector_labels = self.sectors.clone();
        let hover_id = hover.read().clone();
        let measured_draw = measured.clone();
        let hover_draw = hover_id.clone();
        let dial_canvas = canvas(RenderCallback::new(move |ctx| {
            let (w, h) = (ctx.size.width, ctx.size.height);
            measured_draw.set((w, h));
            draw(
                ctx.canvas,
                w,
                h,
                &Frame {
                    dial: Dial::fit(w, h),
                    sweep: sweep_v,
                    reveal: reveal_v,
                    beat: beat_v,
                    scanning,
                    nodes: &nodes,
                    sectors: &sector_labels,
                    hover: hover_draw.as_ref(),
                },
            );
        }))
        .key(frame.get())
        .expanded();

        let mut state = self.state;
        let reach = |dial: &Dial| 24.0 / dial.r;
        let press = move |e: Event<PointerEventData>| {
            let loc = e.element_location();
            let dial = Dial::fit(w, h);
            let (ux, uy) = dial.unit(loc.x as f32, loc.y as f32);
            let hit = state.peek().pick(ux, uy, reach(&dial));
            let mut s = state.write();
            match hit {
                Some(id) => {
                    s.selected = Some(id);
                    s.drawer_open = true;
                }
                None => s.drawer_open = false,
            }
        };
        let moved = move |e: Event<PointerEventData>| {
            let loc = e.element_location();
            let dial = Dial::fit(w, h);
            let (ux, uy) = dial.unit(loc.x as f32, loc.y as f32);
            let hit = state.peek().pick(ux, uy, reach(&dial));
            if *hover.peek() != hit {
                hover.set(hit);
            }
        };
        let sized = {
            let measured = measured.clone();
            move |e: Event<SizedEventData>| {
                let area = e.area;
                let next = (area.width(), area.height());
                measured.set(next);
                if *size.peek() != next {
                    size.set(next);
                }
            }
        };

        let mut layer = rect()
            .content(Content::flex())
            .expanded()
            .on_sized(sized)
            .on_pointer_press(press)
            .on_pointer_move(moved)
            .child(dial_canvas);

        // Sector names around the bezel.
        for (sec, label) in sectors().iter().zip(&self.sectors) {
            let lw = 190.0;
            let (x, y, right) = label_anchor(&dial, sec.mid(), OUTER_R + 0.16, lw);
            let color = if label.active {
                theme::bone()
            } else {
                theme::dust()
            };
            layer = layer.child(
                rect()
                    .content(Content::flex())
                    .position(Position::new_absolute().left(x).top(y - 20.0))
                    .width(Size::px(lw))
                    .cross_align(if right {
                        Alignment::Start
                    } else {
                        Alignment::End
                    })
                    .spacing(3.)
                    .child(
                        rect()
                            .content(Content::flex())
                            .horizontal()
                            .spacing(7.)
                            .cross_align(Alignment::Center)
                            .child(glyph(Glyph::for_kind(sec.kind), theme::copper(), 13.))
                            .child(caps(&label.label, color)),
                    )
                    .child(mono(
                        label.bytes.clone(),
                        theme::TEXT_SM,
                        if label.active {
                            theme::ash()
                        } else {
                            theme::dust()
                        },
                    )),
            );
        }

        // Ring ages along twelve o'clock.
        for (name, rr) in rings() {
            let (x, y) = dial.at(0.05, rr);
            layer = layer.child(
                rect()
                    .content(Content::flex())
                    .position(Position::new_absolute().left(x + 4.0).top(y - 7.0))
                    .child(mono(name, 9.5, theme::dust())),
            );
        }

        // Names for the heaviest few, plus whatever is hovered or selected, never overlapping.
        let mut wanted: Vec<&MapNode> = self
            .nodes
            .iter()
            .filter(|n| n.rank < 5 || n.selected || hover_id.as_ref() == Some(&n.id))
            .collect();
        wanted.sort_by_key(|n| (!(n.selected || hover_id.as_ref() == Some(&n.id)), n.rank));
        let candidates: Vec<LabelBox> = wanted
            .iter()
            .map(|node| {
                let size_px = node_px(node.weight);
                let (x, y) = dial.at(node.angle, node.radius);
                let right = node.angle.rem_euclid(TAU) < std::f32::consts::PI;
                let w = label_width(&node.name, &node.size_label);
                let left = if right {
                    x + size_px + 10.0
                } else {
                    x - size_px - 10.0 - w
                };
                LabelBox {
                    left,
                    top: y - 15.0,
                    w,
                    h: 30.0,
                    right,
                }
            })
            .collect();
        let kept = place_labels(&candidates, &[(dial.cx, dial.cy, dial.r * INNER_R * 0.6)]);
        for (node, b) in wanted
            .iter()
            .zip(&candidates)
            .zip(&kept)
            .filter(|(_, k)| **k)
            .map(|(p, _)| p)
        {
            let tone = theme::tone_color(node.tone);
            let emphasis = node.selected || hover_id.as_ref() == Some(&node.id);
            layer = layer.child(
                rect()
                    .position(Position::new_absolute().left(b.left).top(b.top))
                    .width(Size::px(b.w))
                    .cross_align(if b.right {
                        Alignment::Start
                    } else {
                        Alignment::End
                    })
                    .opacity(if emphasis {
                        1.0
                    } else {
                        (0.8 * reveal_v).max(0.01)
                    })
                    .child(
                        mono(
                            node.name.clone(),
                            theme::TEXT_SM,
                            if emphasis {
                                theme::bone()
                            } else {
                                theme::ash()
                            },
                        )
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis),
                    )
                    .child(mono(node.size_label.clone(), theme::TEXT_XS, tone)),
            );
        }

        layer
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelBox {
    pub left: f32,
    pub top: f32,
    pub w: f32,
    pub h: f32,
    pub right: bool,
}

impl LabelBox {
    fn overlaps(&self, o: &LabelBox) -> bool {
        self.left < o.left + o.w
            && o.left < self.left + self.w
            && self.top < o.top + o.h
            && o.top < self.top + self.h
    }

    fn hits_circle(&self, (cx, cy, r): (f32, f32, f32)) -> bool {
        let nx = cx.clamp(self.left, self.left + self.w);
        let ny = cy.clamp(self.top, self.top + self.h);
        (nx - cx).powi(2) + (ny - cy).powi(2) < r * r
    }
}

/// Mono text is ~0.6em wide per char at 12px.
pub fn label_width(name: &str, size: &str) -> f32 {
    (name.chars().count().max(size.chars().count()) as f32 * 7.3 + 6.0).min(240.0)
}

/// Greedy placement in priority order: a label is kept only if it touches nothing kept before
/// it and no keep-out circle.
pub fn place_labels(candidates: &[LabelBox], keep_out: &[(f32, f32, f32)]) -> Vec<bool> {
    let mut kept: Vec<LabelBox> = Vec::new();
    candidates
        .iter()
        .map(|c| {
            let ok =
                !kept.iter().any(|k| k.overlaps(c)) && !keep_out.iter().any(|z| c.hits_circle(*z));
            if ok {
                kept.push(*c);
            }
            ok
        })
        .collect()
}

/// Plain data for the dial from the full state.
pub fn sector_labels(state: &AppState) -> Vec<SectorLabel> {
    state
        .legend()
        .into_iter()
        .map(|l| SectorLabel {
            kind: l.kind,
            label: l.label,
            bytes: format_bytes(l.bytes),
            count: l.count,
            active: l.active,
        })
        .collect()
}

/// Tone legend chip, used under the dial.
pub fn tone_legend() -> Rect {
    let d = copy::get();
    let chip = |tone: Tone, text: &str| {
        rect()
            .content(Content::flex())
            .horizontal()
            .spacing(7.)
            .cross_align(Alignment::Center)
            .child(
                rect()
                    .content(Content::flex())
                    .width(Size::px(11.))
                    .height(Size::px(11.))
                    .corner_radius(6.)
                    .center()
                    .background(theme::mix(theme::pitch(), theme::tone_color(tone), 0.3))
                    .border(
                        Border::new()
                            .fill(theme::tone_color(tone))
                            .width(1.2)
                            .alignment(BorderAlignment::Inner),
                    )
                    .child(
                        rect()
                            .width(Size::px(3.))
                            .height(Size::px(3.))
                            .corner_radius(2.)
                            .background(theme::tone_color(tone)),
                    ),
            )
            .child(mono(text.to_string(), theme::TEXT_XS, theme::ash()))
    };
    rect()
        .content(Content::flex())
        .horizontal()
        .spacing(18.)
        .child(chip(Tone::Free, d.tone_free))
        .child(chip(Tone::Caution, d.tone_caution))
        .child(chip(Tone::Guarded, d.tone_guarded))
        .child(chip(Tone::Untouchable, d.tone_untouchable))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_never_overlap() {
        let b = |left: f32, top: f32| LabelBox {
            left,
            top,
            w: 100.0,
            h: 30.0,
            right: true,
        };
        let kept = place_labels(
            &[b(0.0, 0.0), b(50.0, 10.0), b(0.0, 40.0), b(500.0, 500.0)],
            &[(520.0, 510.0, 30.0)],
        );
        assert_eq!(kept, vec![true, false, true, false]);
        assert!(label_width("x", "1.0 GB") > label_width("", "x"));
        assert!(label_width(&"y".repeat(200), "") <= 240.0);
    }

    #[test]
    fn dial_roundtrip() {
        let d = Dial::fit(1000.0, 800.0);
        assert!((d.r - 312.0).abs() < 0.01);
        let (x, y) = d.at(0.0, 1.0);
        assert!((x - d.cx).abs() < 1e-3 && (y - (d.cy - d.r)).abs() < 1e-3);
        let (ux, uy) = d.unit(x, y);
        assert!(ux.abs() < 1e-4 && (uy + 1.0).abs() < 1e-4);
        assert!(Dial::fit(10.0, 10.0).r >= 40.0);
        assert!(node_px(1.0) > node_px(0.0));
        let (_, _, right) = label_anchor(&d, 1.0, 1.0, 10.0);
        assert!(right);
        let (lx, _, left) = label_anchor(&d, 4.0, 1.0, 10.0);
        assert!(!left && lx < d.cx);
    }
}
