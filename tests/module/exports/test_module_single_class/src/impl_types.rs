use crate::api::OnlyClass;

#[derive(Default)]
pub struct OnlyImpl;

impl OnlyClass for OnlyImpl {
    fn get_v(&mut self) -> i32 {
        7
    }
}
