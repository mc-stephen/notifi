use super::WebhookSender;
use futures::future::BoxFuture;

pub struct FunctionProvider {
    function_id: String,
}

impl FunctionProvider {
    pub fn new(function_id: String) -> Self {
        Self { function_id }
    }
}

impl WebhookSender for FunctionProvider {
    fn send(
        &self,
        _title: &str,
        _body: &str,
        _brand: &str,
    ) -> BoxFuture<'static, Result<(), String>> {
        let fid = self.function_id.clone();
        Box::pin(async move {
            // Placeholder: Call internal execution engine
            println!("Triggering function: {}", fid);
            Ok(())
        })
    }
}
