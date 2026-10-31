use futures::future::BoxFuture;

pub struct FunctionProvider {
    function_id: String,
}

impl FunctionProvider {
    pub fn new(function_id: String) -> Self {
        Self { function_id }
    }

    /// Placeholder send — the internal execution engine is a later
    /// milestone; the flavor is kept so its day is a form, not a file.
    pub fn send(
        &self,
        _title: &str,
        _body: &str,
        _project: &str,
    ) -> BoxFuture<'static, Result<(), String>> {
        let fid = self.function_id.clone();
        Box::pin(async move {
            // Placeholder: call the internal execution engine.
            println!("Triggering function: {}", fid);
            Ok(())
        })
    }
}
