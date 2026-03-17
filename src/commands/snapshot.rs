use crate::errors::PymgrResult;

pub fn exec_list() -> PymgrResult<()> {
    Ok(())
}

pub fn exec_rollback(_id: Option<&str>) -> PymgrResult<()> {
    Ok(())
}

pub fn exec_diff(_id: &str) -> PymgrResult<()> {
    Ok(())
}

pub fn exec_gc() -> PymgrResult<()> {
    Ok(())
}
