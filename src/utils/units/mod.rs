use crate::*;

pub(super) struct UnitsPlugin;

impl Plugin for UnitsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Metres>()
            .register_type::<MetresPerSecond>();
    }
}

pub(crate) use distance::*;
mod distance {
    use super::*;

    #[derive(Clone, Copy, Deref, DerefMut, Reflect)]
    pub(crate) struct Metres(f32);

    trait IntoMetres {
        fn into_metres(self) -> Metres;
    }
}

pub(crate) use speed::*;
mod speed {
    use super::*;

    #[derive(Clone, Copy, Deref, DerefMut, Reflect)]
    pub(crate) struct MetresPerSecond(f32);

    trait IntoMetresPerSecond {
        fn into_metres_per_second(self) -> MetresPerSecond;
    }
}

pub(crate) use acceleration::*;
mod acceleration {
    use super::*;

    #[derive(Clone, Copy, Deref, DerefMut, Reflect)]
    pub(crate) struct MetresPerSecondSquared(f32);

    trait IntoMetresPerSecondSquared {
        fn into_metres_per_second_squared(self) -> MetresPerSecondSquared;
    }
}
