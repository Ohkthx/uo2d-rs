use sdl2::event::Event;
use sdl2::keyboard::KeyboardState as KeyState;
use sdl2::mouse::MouseButton;
use sdl2::EventPump;

use crate::components::Vec2;

#[derive(Default)]
pub struct MouseState {
    pub position: Vec2,
    pub last_target: Option<Vec2>,
    left_clicked: bool,
    right_clicked: bool,
    tick_delay: u32,
    left_held_ticks: u32,
    right_held_ticks: u32,
}

impl MouseState {
    fn reset(&mut self) {
        self.left_clicked = false;
        self.right_clicked = false;
    }

    pub fn set_delay(&mut self, delay_ticks: u32) {
        self.tick_delay = delay_ticks;
    }

    pub fn clicked(&self) -> bool {
        self.left_clicked || self.right_clicked
    }

    pub fn left_clicked(&self) -> bool {
        self.left_clicked && self.left_held_ticks <= self.tick_delay
    }

    pub fn right_clicked(&self) -> bool {
        self.right_clicked && self.right_held_ticks <= self.tick_delay
    }

    pub fn held(&self) -> bool {
        self.left_held() || self.right_held()
    }

    pub fn left_held(&self) -> bool {
        self.left_held_ticks > self.tick_delay
    }

    pub fn right_held(&self) -> bool {
        self.right_held_ticks > self.tick_delay
    }

    pub fn update(&mut self, event: &Event) {
        match event {
            Event::MouseButtonDown { mouse_btn, .. } => match mouse_btn {
                MouseButton::Left => {
                    if self.left_held_ticks == 0 {
                        self.left_held_ticks = 1;
                    }
                }
                MouseButton::Right => {
                    if self.right_held_ticks == 0 {
                        self.right_held_ticks = 1;
                    }
                }
                _ => {}
            },
            Event::MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                MouseButton::Left => {
                    self.left_clicked =
                        self.left_held_ticks > 0 && self.left_held_ticks <= self.tick_delay;
                    self.left_held_ticks = 0;
                }
                MouseButton::Right => {
                    self.right_clicked =
                        self.right_held_ticks > 0 && self.right_held_ticks <= self.tick_delay;
                    self.right_held_ticks = 0;
                }
                _ => {}
            },
            Event::MouseMotion { x, y, .. } => {
                self.position = Vec2::new(*x as f64, *y as f64);
            }
            _ => {}
        }
    }

    pub fn post_update(&mut self) {
        // Increment ticks for held buttons
        if self.left_held_ticks > 0 {
            self.left_held_ticks += 1;
        }

        if self.right_held_ticks > 0 {
            self.right_held_ticks += 1;
        }

        // Handle target update logic
        if self.clicked() || self.held() {
            self.last_target = Some(self.position);
        }

        // Reset click states at the end of the update cycle
        // self.reset();
    }
}

#[derive(Default)]
pub struct KeyboardState {
    movement_pressed: bool,
    pub w_pressed: bool,
    pub a_pressed: bool,
    pub s_pressed: bool,
    pub d_pressed: bool,
    pub esc_pressed: bool,
}

impl KeyboardState {
    fn reset(&mut self) {
        self.movement_pressed = false;
        self.w_pressed = false;
        self.a_pressed = false;
        self.s_pressed = false;
        self.d_pressed = false;
        self.esc_pressed = false;
    }

    pub fn movement_pressed(&self) -> bool {
        self.w_pressed || self.a_pressed || self.s_pressed || self.d_pressed
    }

    pub fn update(&mut self, event: &KeyState) {
        if event.is_scancode_pressed(sdl2::keyboard::Scancode::Escape) {
            self.esc_pressed = true;
        }
        if event.is_scancode_pressed(sdl2::keyboard::Scancode::W) {
            self.w_pressed = true;
        }
        if event.is_scancode_pressed(sdl2::keyboard::Scancode::A) {
            self.a_pressed = true;
        }
        if event.is_scancode_pressed(sdl2::keyboard::Scancode::S) {
            self.s_pressed = true;
        }
        if event.is_scancode_pressed(sdl2::keyboard::Scancode::D) {
            self.d_pressed = true;
        }
    }
}

#[derive(Default)]
pub struct Input {
    pub mouse: MouseState,
    pub keyboard: KeyboardState,
}

impl Input {
    fn reset(&mut self) {
        self.mouse.reset();
        self.keyboard.reset();
    }

    /// Updates the input, `tick_delay` is used to delay retargetting by ticks.
    pub fn update(&mut self, pump: &mut EventPump) {
        self.reset();

        self.keyboard.update(&pump.keyboard_state());
        for event in pump.poll_iter() {
            self.mouse.update(&event);
        }
        self.mouse.post_update();
    }
}
