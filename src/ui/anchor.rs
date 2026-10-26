//! Shared normalized drag behavior for the live reader and the compact preview.
use crate::message::Message;
use iced::{
    Element, Event, Length, Point, Rectangle, Renderer, Size, Theme,
    advanced::{
        Layout, Renderer as _, Shell, Widget, layout, mouse, renderer,
        widget::{Tree, tree},
    },
};
use vrx::config::Config;
#[derive(Default)]
pub struct Drag {
    pub active: bool,
}
pub fn normalized(point: Point, bounds: Rectangle) -> (f32, f32) {
    (
        ((point.x - bounds.x) / bounds.width.max(1.0)).clamp(0.02, 0.98),
        ((point.y - bounds.y) / bounds.height.max(1.0)).clamp(0.02, 0.98),
    )
}
pub fn update(
    drag: &mut Drag,
    event: &Event,
    bounds: Rectangle,
    cursor: mouse::Cursor,
    shell: &mut Shell<'_, Message>,
) {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            if cursor.is_over(bounds) =>
        {
            drag.active = true
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if drag.active => {
            if let Some(point) = cursor.position() {
                let (x, y) = normalized(point, bounds);
                shell.publish(Message::Anchor(x, y));
            }
            drag.active = false;
            shell.capture_event();
            return;
        }
        Event::Window(iced::window::Event::Unfocused) => {
            drag.active = false;
            return;
        }
        _ => {}
    }
    if drag.active
        && matches!(
            event,
            Event::Mouse(
                mouse::Event::CursorMoved { .. } | mouse::Event::ButtonPressed(mouse::Button::Left)
            )
        )
        && let Some(point) = cursor.position()
    {
        let (x, y) = normalized(point, bounds);
        shell.publish(Message::Anchor(x, y));
        shell.capture_event();
    }
}
pub struct AnchorPad<'a> {
    pub config: &'a Config,
}
impl Widget<Message, Theme, Renderer> for AnchorPad<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(148.0))
    }
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Drag>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(Drag::default())
    }
    fn layout(&mut self, _: &mut Tree, _: &Renderer, limits: &layout::Limits) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fill, 148.0, Size::new(300.0, 148.0)))
    }
    fn draw(
        &self,
        _: &Tree,
        r: &mut Renderer,
        _: &Theme,
        _: &renderer::Style,
        layout: Layout<'_>,
        _: mouse::Cursor,
        _: &Rectangle,
    ) {
        let b = layout.bounds();
        let (_, foreground, _) = super::theme::palette(self.config);
        let accent = super::theme::focal_color(self.config);
        r.fill_quad(
            renderer::Quad {
                bounds: b,
                border: iced::Border {
                    color: foreground.scale_alpha(0.25),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..renderer::Quad::default()
            },
            foreground.scale_alpha(0.04),
        );
        let p = Point::new(
            b.x + b.width * self.config.focal_x,
            b.y + b.height * self.config.vertical,
        );
        for bounds in [
            Rectangle::new(Point::new(p.x - 12.0, p.y - 1.0), Size::new(24.0, 2.0)),
            Rectangle::new(Point::new(p.x - 1.0, p.y - 12.0), Size::new(2.0, 24.0)),
        ] {
            r.fill_quad(
                renderer::Quad {
                    bounds,
                    ..renderer::Quad::default()
                },
                accent,
            );
        }
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _: &Renderer,
        _: &mut dyn iced::advanced::Clipboard,
        shell: &mut Shell<'_, Message>,
        _: &Rectangle,
    ) {
        update(
            tree.state.downcast_mut::<Drag>(),
            event,
            layout.bounds(),
            cursor,
            shell,
        );
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _: &Rectangle,
        _: &Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<Drag>().active {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::None
        }
    }
}
impl<'a> From<AnchorPad<'a>> for Element<'a, Message> {
    fn from(pad: AnchorPad<'a>) -> Self {
        Element::new(pad)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drag_coordinates_scale_and_clamp() {
        let b = Rectangle::new(Point::new(100.0, 200.0), Size::new(400.0, 200.0));
        assert_eq!(normalized(Point::new(300.0, 300.0), b), (0.5, 0.5));
        assert_eq!(normalized(Point::new(-100.0, 1000.0), b), (0.02, 0.98));
    }
    #[test]
    fn dragging_emits_normalized_positions_and_releases_capture() {
        let b = Rectangle::new(Point::new(100.0, 100.0), Size::new(200.0, 100.0));
        let mut drag = Drag::default();
        let mut messages = Vec::new();
        update(
            &mut drag,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            b,
            mouse::Cursor::Available(Point::new(200.0, 150.0)),
            &mut Shell::new(&mut messages),
        );
        assert!(drag.active);
        assert!(
            matches!(messages.last(),Some(Message::Anchor(x,y)) if (*x-0.5).abs()<0.001&&(*y-0.5).abs()<0.001)
        );
        update(
            &mut drag,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            b,
            mouse::Cursor::Available(Point::new(200.0, 150.0)),
            &mut Shell::new(&mut messages),
        );
        assert!(!drag.active);
    }
}
