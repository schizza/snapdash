use iced::widget::canvas::{Cache, Canvas, Geometry, Path, Program, Stroke, Style};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, mouse};

/// Stateless sparkline that paints a list of values into its bounds.
/// Min/max are auto-fit to the data; caller provides Y-color.
pub struct Sparkline {
    data: Vec<f32>,
    color: Color,
    cache: Cache,
}

impl Sparkline {
    pub fn new(data: Vec<f32>, color: Color) -> Self {
        Self {
            data,
            color,
            cache: Cache::default(),
        }
    }

    pub fn view<'a, Message: 'a>(self) -> Element<'a, Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<Message> Program<Message> for Sparkline {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            if self.data.len() < 2 {
                return;
            }

            let (min, max) = self
                .data
                .iter()
                .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));

            let range = (max - min).max(1.0); // div-by-zero guard
            let w = bounds.width;
            let h = bounds.height;
            let step = w / (self.data.len() - 1) as f32;

            let path = Path::new(|p| {
                let first_y = h - ((self.data[0] - min) / range) * h;
                p.move_to(Point::new(0.0, first_y));

                for (i, &v) in self.data.iter().enumerate().skip(1) {
                    let x = i as f32 * step;
                    let y = h - ((v - min) / range) * h;
                    p.line_to(Point::new(x, y));
                }
            });

            frame.stroke(
                &path,
                Stroke {
                    style: Style::Solid(self.color),
                    width: 1.5,
                    ..Default::default()
                },
            );
        });

        vec![geometry]
    }
}
