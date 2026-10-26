//! A small custom widget: Iced's shaped paragraph span geometry anchors the ORP.
//! The widget tree retains one paragraph; UI rebuilds do not re-shape static text.
use crate::{
    message::Message,
    ui::anchor::{self, Drag},
};
use iced::advanced::Renderer as BaseRenderer;
use iced::{
    Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Theme,
    advanced::{
        Layout, Widget, layout, mouse, renderer,
        text::{self, Paragraph, Renderer as TextRenderer},
        widget::{Tree, tree},
    },
    alignment,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use vrx::config::{Config, GuideShape};
type Shaped = <Renderer as TextRenderer>::Paragraph;

#[derive(Default)]
struct Cache {
    paragraph: Shaped,
    word: String,
    font: Font,
    size: f32,
    accent: Color,
    focus: (usize, usize, bool),
    center: f32,
    width: f32,
    height: f32,
    drag: Drag,
}
pub struct Reader<'a> {
    pub word: &'a str,
    pub focus: (usize, usize, bool),
    pub config: &'a Config,
    pub centered: bool,
    pub editing_anchor: bool,
    pub epoch: Instant,
    /// Last renderer command submission (not scanout or GPU completion).
    pub submission: Arc<AtomicU64>,
}

impl Widget<Message, Theme, Renderer> for Reader<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Cache>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(Cache::default())
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let cache = tree.state.downcast_mut::<Cache>();
        if !self.editing_anchor {
            cache.drag.active = false;
        }
        let font = super::theme::font(self.config);
        let effective_size = self.config.font_size / self.config.gui_scale();
        let focus = (
            self.focus.0,
            self.focus.1,
            self.focus.2 || !self.config.focal_enabled,
        );
        let accent = super::theme::focal_color(self.config);
        if cache.word != self.word
            || cache.font != font
            || cache.size != effective_size
            || cache.accent != accent
            || cache.focus != focus
        {
            self.word.clone_into(&mut cache.word);
            cache.font = font;
            cache.size = effective_size;
            cache.accent = accent;
            cache.focus = focus;
            let (start, end, whole) = focus;
            let spans: [text::Span<'_, (), Font>; 3] = if whole {
                [
                    text::Span::new(self.word),
                    text::Span::new(""),
                    text::Span::new(""),
                ]
            } else {
                [
                    text::Span::new(self.word.get(..start).unwrap_or("")),
                    text::Span::new(self.word.get(start..end).unwrap_or("")).color(accent),
                    text::Span::new(self.word.get(end..).unwrap_or("")),
                ]
            };
            cache.paragraph = Shaped::with_spans(text::Text {
                content: &spans,
                bounds: Size::new(100_000.0, effective_size * 2.0),
                size: Pixels(effective_size),
                line_height: text::LineHeight::Relative(1.4),
                font,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::None,
            });
            cache.width = cache.paragraph.min_width();
            cache.height = cache.paragraph.min_height();
            cache.center = if whole {
                cache.width / 2.0
            } else {
                cache
                    .paragraph
                    .span_bounds(1)
                    .first()
                    .map(|r| r.x + r.width / 2.0)
                    .unwrap_or(cache.width / 2.0)
            };
        }
        layout::Node::new(limits.resolve(Length::Fill, Length::Fill, Size::new(400.0, 260.0)))
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let cache = tree.state.downcast_ref::<Cache>();
        let bounds = layout.bounds();
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        let (_, foreground, _) = super::theme::palette(self.config);
        let accent = super::theme::focal_color(self.config);
        let focal_x = if self.centered {
            0.5
        } else {
            self.config.focal_x
        };
        let focal_y = if self.centered {
            0.5
        } else {
            self.config.vertical
        };
        let center = if self.centered {
            cache.width / 2.0
        } else {
            cache.center
        };
        let anchor = Point::new(
            bounds.x + bounds.width * focal_x,
            bounds.y + bounds.height * focal_y,
        );
        let guide = (self.config.guide && self.config.focal_enabled).then(|| {
            GuideMetrics::new(
                self.config.guide_shape,
                cache.size,
                self.config.guide_size_percent,
                self.config.guide_gap_percent,
            )
        });
        let (left, right, height) = if let Some(guide) = guide {
            (
                center.max(center - cache.center + guide.width / 2.0),
                (cache.width - center).max(cache.center - center + guide.width / 2.0),
                cache.height + 2.0 * (guide.gap + guide.height),
            )
        } else {
            (center, cache.width - center, cache.height)
        };
        // Include the guides in fitting so their size and gap scale with the
        // text, even for a large font or a focal point near the window edge.
        let scale = fit_scale(
            bounds.width,
            bounds.height,
            focal_x,
            focal_y,
            left,
            left + right,
            height,
        );
        let origin = Point::new(
            anchor.x - center * scale,
            anchor.y - cache.height * scale / 2.0,
        );
        renderer.with_layer(clip, |renderer| {
            renderer.with_transformation(
                iced::Transformation::translate(origin.x, origin.y)
                    * iced::Transformation::scale(scale),
                |renderer| {
                    renderer.fill_paragraph(
                        &cache.paragraph,
                        Point::ORIGIN,
                        foreground,
                        Rectangle::new(
                            Point::new(-100_000.0, -100_000.0),
                            Size::new(200_000.0, 200_000.0),
                        ),
                    );
                },
            );
            if let Some(guide) = guide {
                let focal = Point::new(origin.x + cache.center * scale, anchor.y);
                for bounds in guide.rectangles(focal, cache.height, scale) {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds,
                            border: iced::Border {
                                radius: if guide.rounded {
                                    bounds.width / 2.0
                                } else {
                                    0.0
                                }
                                .into(),
                                ..iced::Border::default()
                            },
                            ..renderer::Quad::default()
                        },
                        accent,
                    );
                }
            }
        });
        if self.config.diagnostics {
            self.submission.store(
                self.epoch.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
                Ordering::Relaxed,
            );
        }
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _: &Renderer,
        _: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _: &Rectangle,
    ) {
        if self.editing_anchor {
            anchor::update(
                &mut tree.state.downcast_mut::<Cache>().drag,
                event,
                layout.bounds(),
                cursor,
                shell,
            );
        }
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _: &Rectangle,
        _: &Renderer,
    ) -> mouse::Interaction {
        if self.editing_anchor && tree.state.downcast_ref::<Cache>().drag.active {
            mouse::Interaction::Grabbing
        } else if self.editing_anchor && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::None
        }
    }
}
/// All dimensions are relative to the effective reading font, before the
/// shared word-fitting transform. GUI scaling therefore cannot enlarge guides
/// independently of the word.
#[derive(Debug, Clone, Copy)]
struct GuideMetrics {
    width: f32,
    height: f32,
    gap: f32,
    rounded: bool,
}
impl GuideMetrics {
    fn new(shape: GuideShape, em: f32, size_percent: u32, gap_percent: u32) -> Self {
        let unit = em * size_percent.clamp(25, 400) as f32 / 100.0;
        let (width, height) = match shape {
            GuideShape::Ticks => (unit * 0.025, unit * 0.1),
            GuideShape::Dots => (unit * 0.08, unit * 0.08),
            GuideShape::Lines => (unit * 0.24, unit * 0.025),
        };
        Self {
            width,
            height,
            gap: em * gap_percent.min(100) as f32 / 100.0,
            rounded: shape == GuideShape::Dots,
        }
    }
    fn rectangles(self, focal: Point, word_height: f32, scale: f32) -> [Rectangle; 2] {
        let x = focal.x - self.width * scale / 2.0;
        let size = Size::new(self.width * scale, self.height * scale);
        [
            Rectangle::new(
                Point::new(
                    x,
                    focal.y - (word_height / 2.0 + self.gap + self.height) * scale,
                ),
                size,
            ),
            Rectangle::new(
                Point::new(x, focal.y + (word_height / 2.0 + self.gap) * scale),
                size,
            ),
        ]
    }
}

pub fn fit_scale(
    width: f32,
    height: f32,
    focal_x: f32,
    focal_y: f32,
    center: f32,
    word_width: f32,
    word_height: f32,
) -> f32 {
    let left = (width * focal_x - 24.0).max(1.0) / center.max(1.0);
    let right = (width * (1.0 - focal_x) - 24.0).max(1.0) / (word_width - center).max(1.0);
    let above = (height * focal_y - 2.0).max(1.0) / (word_height / 2.0).max(1.0);
    let below = (height * (1.0 - focal_y) - 2.0).max(1.0) / (word_height / 2.0).max(1.0);
    left.min(right).min(above).min(below).clamp(0.001, 1.0)
}
impl<'a> From<Reader<'a>> for Element<'a, Message> {
    fn from(reader: Reader<'a>) -> Self {
        Element::new(reader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_anchor_when_resized() {
        for width in [300.0, 800.0, 1920.0] {
            for focus in [0.2, 0.45, 0.8] {
                let s = fit_scale(width, 400.0, focus, 0.5, 240.0, 1000.0, 100.0);
                let anchor = width * focus;
                let origin = anchor - 240.0 * s;
                assert!((origin + 240.0 * s - anchor).abs() < 0.001);
                assert!(origin >= 23.9);
                assert!(origin + 1000.0 * s <= width - 23.9);
            }
        }
    }
    #[test]
    fn proportional_shaping_and_focus_graphemes_have_real_geometry() {
        let shape = |word: &str, start: usize, end: usize, size: f32| {
            let spans: [text::Span<'_, (), Font>; 3] = [
                text::Span::new(word.get(..start).unwrap_or("")),
                text::Span::new(word.get(start..end).unwrap_or("")).color(Color::WHITE),
                text::Span::new(word.get(end..).unwrap_or("")),
            ];
            Shaped::with_spans(text::Text {
                content: &spans,
                bounds: Size::new(10000.0, 200.0),
                size: Pixels(size),
                line_height: text::LineHeight::Relative(1.4),
                font: Font::DEFAULT,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::None,
            })
        };
        let narrow = shape("iii", 1, 2, 70.0);
        let wide = shape("WWW", 1, 2, 70.0);
        assert!(wide.min_width() > narrow.min_width() * 1.5);
        for (word, start, end) in [("office", 2, 3), ("e\u{301}clair", 0, 3), ("‘wide,’", 3, 4)]
        {
            let shaped = shape(word, start, end, 70.0);
            let focus = shaped.span_bounds(1);
            assert!(!focus.is_empty(), "{word} has no focus geometry");
            assert!(focus.iter().all(|r| r.width.is_finite() && r.width > 0.0));
        }
        let normal = shape("wide", 1, 2, 40.0);
        let large = shape("wide", 1, 2, 80.0);
        assert!((large.min_width() / normal.min_width() - 2.0).abs() < 0.1);
    }
    #[test]
    fn vertical_positions_near_edges_never_clip_text() {
        for y in [0.02, 0.15, 0.5, 0.85, 0.98] {
            let scale = fit_scale(800.0, 400.0, 0.5, y, 120.0, 300.0, 500.0);
            let top = 400.0 * y - 250.0 * scale;
            assert!(top >= 1.9);
            assert!(top + 500.0 * scale <= 398.1);
        }
    }
    #[test]
    fn focused_revision_guides_scale_with_text_and_fit_at_edges() {
        for shape in GuideShape::ALL {
            let normal = GuideMetrics::new(shape, 76.0, 100, 12);
            let large = GuideMetrics::new(shape, 152.0, 100, 12);
            assert!((large.width - 2.0 * normal.width).abs() < 0.001);
            assert!((large.height - 2.0 * normal.height).abs() < 0.001);
            assert!((large.gap - 2.0 * normal.gap).abs() < 0.001);
            let enlarged_gui = GuideMetrics::new(shape, 38.0, 100, 12);
            assert!((enlarged_gui.width * 2.0 - normal.width).abs() < 0.001);
            for font in [20.0, 76.0, 360.0] {
                for anchor in [0.02, 0.5, 0.98] {
                    let guide = GuideMetrics::new(shape, font, 400, 100);
                    let word_height = font * 1.4;
                    let height = word_height + 2.0 * (guide.gap + guide.height);
                    let scale = fit_scale(800.0, 400.0, 0.5, anchor, 100.0, 200.0, height);
                    for r in guide.rectangles(Point::new(400.0, 400.0 * anchor), word_height, scale)
                    {
                        assert!(r.y >= 1.99);
                        assert!(r.y + r.height <= 398.01);
                    }
                }
            }
        }
    }
}
