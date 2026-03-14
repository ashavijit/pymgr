use crate::errors::PymgrResult;

pub async fn exec() -> PymgrResult<()> {
    crate::self_update::self_update().await
}
