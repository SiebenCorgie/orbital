use atomic_float::AtomicF32;
use nih_plug::prelude::{Editor, GuiContext};
use nih_plug_iced::{
    canvas, create_iced_editor, executor, time, widget, Canvas, Command, Element, IcedEditor,
    IcedState, Length, Point, Settings, Subscription, WindowQueue,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::OrbitalParams;

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<IcedState> {
    IcedState::from_size(200, 150)
}

pub(crate) fn create(
    params: Arc<OrbitalParams>,
    peak_meter: Arc<AtomicF32>,
    editor_state: Arc<IcedState>,
) -> Option<Box<dyn Editor>> {
    create_iced_editor::<OrbitalEditor>(editor_state, (params, peak_meter))
}

struct OrbitalEditor {
    params: Arc<OrbitalParams>,
    context: Arc<dyn GuiContext>,

    state: State,
    pitch: Arc<AtomicF32>,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Tick(Instant),
}

impl IcedEditor for OrbitalEditor {
    type Executor = executor::Default;
    type Message = Message;
    type InitializationFlags = (Arc<OrbitalParams>, Arc<AtomicF32>);

    fn new(
        (params, pitch): Self::InitializationFlags,
        context: Arc<dyn GuiContext>,
    ) -> (Self, Command<Self::Message>) {
        let editor = OrbitalEditor {
            params,
            context,
            state: State::new(),
            pitch,
        };

        (editor, Command::none())
    }

    fn context(&self) -> &dyn GuiContext {
        self.context.as_ref()
    }

    fn update(
        &mut self,
        _window: &mut WindowQueue,
        message: Self::Message,
    ) -> Command<Self::Message> {
        match message {
            Message::Tick(instant) => {
                self.state.update(instant);
            }
        }

        Command::none()
    }

    fn background_color(&self) -> nih_plug_iced::Color {
        nih_plug_iced::Color {
            r: 0.98,
            g: 0.98,
            b: 0.98,
            a: 1.0,
        }
    }

    fn view(&self) -> Element<Message> {
        Canvas::new(&self.state)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::time::every(time::Duration::from_millis(10)).map(Message::Tick)
    }
}

#[derive(Debug)]
struct State {
    space_cache: canvas::Cache,
    system_cache: canvas::Cache,
    start: Instant,
    now: Instant,
    stars: Vec<(Point, f32)>,
}

impl State {
    const SUN_RADIUS: f32 = 70.0;
    const ORBIT_RADIUS: f32 = 150.0;
    const EARTH_RADIUS: f32 = 12.0;
    const MOON_RADIUS: f32 = 4.0;
    const MOON_DISTANCE: f32 = 28.0;

    pub fn new() -> State {
        let now = Instant::now();
        let (width, height) = Settings::default().size;

        State {
            space_cache: Default::default(),
            system_cache: Default::default(),
            start: now,
            now,
            stars: Self::generate_stars(width, height),
        }
    }

    pub fn update(&mut self, now: Instant) {
        self.now = now;
        self.system_cache.clear();
    }

    fn generate_stars(width: u32, height: u32) -> Vec<(Point, f32)> {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        (0..100)
            .map(|_| {
                (
                    Point::new(
                        rng.gen_range((-(width as f32) / 2.0)..(width as f32 / 2.0)),
                        rng.gen_range((-(height as f32) / 2.0)..(height as f32 / 2.0)),
                    ),
                    rng.gen_range(0.5..1.0),
                )
            })
            .collect()
    }
}

impl<Message> canvas::Program<Message> for State {
    type State = ();
    fn draw(
        &self,
        _state: &Self::State,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<canvas::Geometry> {
        use std::f32::consts::PI;

        let background = self.space_cache.draw(bounds.size(), |frame| {
            let stars = Path::new(|path| {
                for (p, size) in &self.stars {
                    path.rectangle(*p, Size::new(*size, *size));
                }
            });

            frame.translate(frame.center() - Point::ORIGIN);
            frame.fill(&stars, Color::WHITE);
        });

        let system = self.system_cache.draw(bounds.size(), |frame| {
            let center = frame.center();

            let sun = Path::circle(center, Self::SUN_RADIUS);
            let orbit = Path::circle(center, Self::ORBIT_RADIUS);

            frame.fill(&sun, Color::from_rgb8(0xF9, 0xD7, 0x1C));
            frame.stroke(
                &orbit,
                Stroke {
                    style: stroke::Style::Solid(Color::from_rgba8(0, 153, 255, 0.1)),
                    width: 1.0,
                    line_dash: canvas::LineDash {
                        offset: 0,
                        segments: &[3.0, 6.0],
                    },
                    ..Stroke::default()
                },
            );

            let elapsed = self.now - self.start;
            let rotation = (2.0 * PI / 60.0) * elapsed.as_secs() as f32
                + (2.0 * PI / 60_000.0) * elapsed.subsec_millis() as f32;

            frame.with_save(|frame| {
                frame.translate(Vector::new(center.x, center.y));
                frame.rotate(rotation);
                frame.translate(Vector::new(Self::ORBIT_RADIUS, 0.0));

                let earth = Path::circle(Point::ORIGIN, Self::EARTH_RADIUS);

                let earth_fill = Gradient::linear(gradient::Position::Absolute {
                    start: Point::new(-Self::EARTH_RADIUS, 0.0),
                    end: Point::new(Self::EARTH_RADIUS, 0.0),
                })
                .add_stop(0.2, Color::from_rgb(0.15, 0.50, 1.0))
                .add_stop(0.8, Color::from_rgb(0.0, 0.20, 0.47))
                .build()
                .expect("Build Earth fill gradient");

                frame.fill(&earth, earth_fill);

                frame.with_save(|frame| {
                    frame.rotate(rotation * 10.0);
                    frame.translate(Vector::new(0.0, Self::MOON_DISTANCE));

                    let moon = Path::circle(Point::ORIGIN, Self::MOON_RADIUS);
                    frame.fill(&moon, Color::WHITE);
                });
            });
        });

        vec![background, system]
    }
}
