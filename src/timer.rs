use std::time::Duration;

use uuid::Uuid;

/// Data that is attached to the timer.
#[derive(Debug)]
pub enum TimerData {
    Empty,
    EntityDelete(Uuid),
}

/// Allows for tracking of various time sensitive events.
#[derive(Debug)]
pub struct Timer {
    /// The tick the timer was created on.
    pub start: u64,
    /// Length of time in ticks.
    pub span: u64,
    /// Data attached to the timer.
    pub data: TimerData,
}

impl Timer {
    /// Creates a new timer.
    fn new(start: u64, span: u64, data: TimerData) -> Self {
        Self { start, span, data }
    }

    /// Checks if a timer is expired.
    #[inline]
    fn is_expired(&self, current_tick: u64) -> bool {
        self.start + self.span <= current_tick
    }
}

/// Manages all created timers.
pub struct TimerManager {
    /// Sort vector of timers, more recently expiring are in front.
    timers: Vec<Timer>,
    /// Current tick.
    tick: u64,
    /// Duration of a tick for the server.
    server_tick: Duration,
    /// Duration of a tick for the client.
    client_tick: Duration,
}

impl TimerManager {
    const SERVER_TICKS_PER_SECOND: f32 = 180.0;
    const SERVER_TICK_RATE_MICROSECOND: f32 = 1_000_000.0 / Self::SERVER_TICKS_PER_SECOND;

    const CLIENT_TICKS_PER_SECOND: f32 = Self::SERVER_TICKS_PER_SECOND / 3.0;
    const CLIENT_TICK_RATE_MICROSECOND: f32 = 1_000_000.0 / Self::CLIENT_TICKS_PER_SECOND;

    /// Creates a new manager for timers.
    pub fn new() -> Self {
        Self {
            timers: Vec::new(),
            tick: 0,
            server_tick: Duration::from_micros(Self::SERVER_TICK_RATE_MICROSECOND.round() as u64),
            client_tick: Duration::from_micros(Self::CLIENT_TICK_RATE_MICROSECOND.round() as u64),
        }
    }

    /// Current tick the server is on.
    #[allow(dead_code)]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Amount of time per server tick.
    pub fn server_tick_time(&self) -> Duration {
        self.server_tick
    }

    /// Amount of time per client tick.
    pub fn client_tick_time(&self) -> Duration {
        self.client_tick
    }

    /// Removes and returns timers that have completed.
    pub fn update(&mut self) -> Vec<Timer> {
        self.tick += 1;

        // Find the index of the first non-expired timer
        let first_active_index = self
            .timers
            .iter()
            .position(|timer| !timer.is_expired(self.tick))
            .unwrap_or(self.timers.len()); // If all are expired or none, take appropriate action

        // Split the timers at the found index, taking all expired timers out
        let expired_timers = self
            .timers
            .drain(..first_active_index)
            .collect::<Vec<Timer>>();

        expired_timers
    }

    /// Adds a new timer, where span is number of seconds the timer should exist for.
    pub fn add_timer_sec(&mut self, span: f32, data: TimerData, is_server: bool) {
        // Calculate the number of ticks based on whether it's a server or client timer.
        let ticks_per_second = if is_server {
            Self::SERVER_TICKS_PER_SECOND
        } else {
            Self::CLIENT_TICKS_PER_SECOND
        };

        // Add the timer with the calculated number of ticks
        let span_ticks = (span * ticks_per_second).round() as u64;
        self.add_timer_tick(span_ticks, data);
    }

    /// Adds a new timer, where span is number of ticks the timer should exist for.
    pub fn add_timer_tick(&mut self, span: u64, data: TimerData) {
        let new_timer = Timer::new(self.tick, span, data);
        let position = self
            .timers
            .iter()
            .position(|timer| timer.start + timer.span > new_timer.start + new_timer.span)
            .unwrap_or(self.timers.len());
        self.timers.insert(position, new_timer);
    }
}
