//! Utility functions for accessing a [`Repository`].

use gix::actor::IdentityRef;

use crate::somehow;

// TODO Remove this function
pub fn format_actor(author: IdentityRef<'_>) -> somehow::Result<String> {
    let mut buffer = vec![];
    author.trim().write_to(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}
