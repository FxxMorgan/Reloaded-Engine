use serde::{Deserialize, Serialize};

/// Android Surface lifecycle state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SurfaceState {
    #[default]
    Uninitialized,
    Created {
        width: i32,
        height: i32,
    },
    Ready {
        width: i32,
        height: i32,
    },
    Destroyed,
}

/// Android Activity / App lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ActivityState {
    #[default]
    Idle,
    Active,
    Paused,
    Stopped,
}

/// Lifecycle state machine coordinator preventing background freezes and deadlocks.
#[derive(Debug, Clone, Default)]
pub struct SurfaceLifecycle {
    surface_state: SurfaceState,
    activity_state: ActivityState,
    surface_lost_count: u64,
    surface_recreate_count: u64,
}

impl SurfaceLifecycle {
    /// Creates a new uninitialized lifecycle state machine.
    pub fn new() -> Self {
        Self {
            surface_state: SurfaceState::Uninitialized,
            activity_state: ActivityState::Idle,
            surface_lost_count: 0,
            surface_recreate_count: 0,
        }
    }

    /// Handles Android `surfaceCreated` callback.
    pub fn on_surface_created(&mut self, width: i32, height: i32) {
        if matches!(self.surface_state, SurfaceState::Destroyed) {
            self.surface_recreate_count += 1;
        }

        self.surface_state = SurfaceState::Created { width, height };
        if self.activity_state == ActivityState::Idle {
            self.activity_state = ActivityState::Active;
        }
    }

    /// Handles Android `surfaceChanged` callback with new width and height.
    pub fn on_surface_changed(&mut self, width: i32, height: i32) {
        if width > 0 && height > 0 {
            self.surface_state = SurfaceState::Ready { width, height };
        } else {
            self.surface_state = SurfaceState::Created { width, height };
        }
    }

    /// Handles Android `surfaceDestroyed` callback (e.g. app minimized or backgrounded).
    /// Prevents background freezes by invalidating the surface immediately.
    pub fn on_surface_destroyed(&mut self) {
        self.surface_state = SurfaceState::Destroyed;
        self.surface_lost_count += 1;
    }

    /// Pauses game execution (e.g. Activity `onPause`).
    pub fn pause(&mut self) {
        self.activity_state = ActivityState::Paused;
    }

    /// Resumes game execution (e.g. Activity `onResume`).
    pub fn resume(&mut self) {
        self.activity_state = ActivityState::Active;
    }

    /// Stops game execution.
    pub fn stop(&mut self) {
        self.activity_state = ActivityState::Stopped;
    }

    /// Returns true if the surface is valid and the activity is active for rendering.
    #[inline]
    pub fn can_render(&self) -> bool {
        matches!(self.surface_state, SurfaceState::Ready { width, height } if width > 0 && height > 0)
            && self.activity_state == ActivityState::Active
    }

    /// Returns true if the surface is ready (even if currently paused).
    #[inline]
    pub fn is_surface_ready(&self) -> bool {
        matches!(self.surface_state, SurfaceState::Ready { width, height } if width > 0 && height > 0)
    }

    /// Returns true if the surface is destroyed or uninitialized.
    #[inline]
    pub fn is_surface_lost(&self) -> bool {
        matches!(self.surface_state, SurfaceState::Destroyed | SurfaceState::Uninitialized)
    }

    /// Current surface state.
    #[inline]
    pub fn surface_state(&self) -> SurfaceState {
        self.surface_state
    }

    /// Current activity state.
    #[inline]
    pub fn activity_state(&self) -> ActivityState {
        self.activity_state
    }

    /// Dimensions if ready or created.
    pub fn dimensions(&self) -> Option<(i32, i32)> {
        match self.surface_state {
            SurfaceState::Created { width, height } | SurfaceState::Ready { width, height } => {
                Some((width, height))
            }
            _ => None,
        }
    }

    pub fn surface_lost_count(&self) -> u64 {
        self.surface_lost_count
    }

    pub fn surface_recreate_count(&self) -> u64 {
        self.surface_recreate_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_full_transitions() {
        let mut lc = SurfaceLifecycle::new();
        assert_eq!(lc.surface_state(), SurfaceState::Uninitialized);
        assert_eq!(lc.activity_state(), ActivityState::Idle);
        assert!(!lc.can_render());
        assert!(lc.is_surface_lost());

        // 1. Surface Created
        lc.on_surface_created(1920, 1080);
        assert_eq!(
            lc.surface_state(),
            SurfaceState::Created {
                width: 1920,
                height: 1080
            }
        );
        assert_eq!(lc.activity_state(), ActivityState::Active);
        assert!(!lc.can_render(), "Not ready until surfaceChanged");

        // 2. Surface Changed -> Ready
        lc.on_surface_changed(1920, 1080);
        assert_eq!(
            lc.surface_state(),
            SurfaceState::Ready {
                width: 1920,
                height: 1080
            }
        );
        assert!(lc.can_render());
        assert!(lc.is_surface_ready());
        assert!(!lc.is_surface_lost());

        // 3. User goes to home screen -> Surface Destroyed (Background Freeze Prevention)
        lc.on_surface_destroyed();
        assert_eq!(lc.surface_state(), SurfaceState::Destroyed);
        assert!(!lc.can_render(), "Must not render to destroyed surface!");
        assert!(lc.is_surface_lost());
        assert_eq!(lc.surface_lost_count(), 1);

        // 4. Activity pause
        lc.pause();
        assert_eq!(lc.activity_state(), ActivityState::Paused);
        assert!(!lc.can_render());

        // 5. User returns -> Activity resume
        lc.resume();
        assert_eq!(lc.activity_state(), ActivityState::Active);
        // Surface is still destroyed until Android calls surfaceCreated
        assert!(!lc.can_render());

        // 6. Surface Recreated
        lc.on_surface_created(1920, 1080);
        assert_eq!(lc.surface_recreate_count(), 1);
        lc.on_surface_changed(1920, 1080);
        assert!(lc.can_render());
        assert_eq!(lc.dimensions(), Some((1920, 1080)));
    }

    #[test]
    fn test_background_freeze_prevention_on_pause() {
        let mut lc = SurfaceLifecycle::new();
        lc.on_surface_created(800, 480);
        lc.on_surface_changed(800, 480);
        assert!(lc.can_render());

        // Paused while surface is technically still intact
        lc.pause();
        assert_eq!(lc.activity_state(), ActivityState::Paused);
        assert!(!lc.can_render(), "Rendering must halt during pause!");
        assert!(lc.is_surface_ready());

        // Resumed
        lc.resume();
        assert!(lc.can_render());
    }

    #[test]
    fn test_zero_dimensions_handling() {
        let mut lc = SurfaceLifecycle::new();
        lc.on_surface_created(0, 0);
        lc.on_surface_changed(0, 0);
        assert!(!lc.can_render(), "Cannot render with 0x0 viewport");
        assert!(!lc.is_surface_ready());
    }
}
