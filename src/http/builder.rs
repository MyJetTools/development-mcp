use std::sync::Arc;

use my_http_server::controllers::ControllersMiddleware;

use crate::app::AppContext;

pub fn build_controllers(app: &Arc<AppContext>) -> ControllersMiddleware {
    let mut result = ControllersMiddleware::new(None, None);

    result.register_get_action(Arc::new(
        crate::http::controllers::ai_docs::GetAiDocsIndexAction::new(app.clone()),
    ));

    result.register_get_action(Arc::new(
        crate::http::controllers::ai_docs::GetAiDocsYamlAction::new(app.clone()),
    ));

    result.register_get_action(Arc::new(
        crate::http::controllers::ai_docs::GetAiDocAction::new(app.clone()),
    ));

    result
}
