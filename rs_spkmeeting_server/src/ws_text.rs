use actix::prelude::*;

/// Lightweight message type used for delivering text to WsConn instances
#[derive(Message, Clone)]
#[rtype(result = "()")] 
pub struct WsText(pub String);
