use crate::models::execution::ExecutionResult;
use crate::models::menu::MenuItem;

pub trait MenuItemProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    fn get_menu_items(&self) -> Vec<MenuItem>;
    fn refresh(&mut self);
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub trait ExecutionHandler {
    fn can_handle(&self, item: &MenuItem) -> bool;
    fn execute(&self, item: &MenuItem, context: Option<&str>) -> ExecutionResult;
}
