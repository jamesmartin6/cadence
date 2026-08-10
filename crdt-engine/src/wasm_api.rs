use wasm_bindgen::prelude::*;

use crate::doc::Doc;
use crate::op::Operation;

/// The WASM-facing wrapper around [`Doc`], consumed directly from TypeScript.
///
/// Operations are handed back and forth as `JsValue` (plain JSON-shaped objects via
/// `serde-wasm-bindgen`) so they can be sent over the WebSocket to the relay server
/// unchanged, and so remote operations received from the socket can be fed straight
/// back in without the frontend needing to know anything about the CRDT internals.
#[wasm_bindgen]
pub struct CrdtDoc {
    inner: Doc,
}

#[wasm_bindgen]
impl CrdtDoc {
    #[wasm_bindgen(constructor)]
    pub fn new(site_id: u32) -> CrdtDoc {
        CrdtDoc {
            inner: Doc::new(site_id),
        }
    }

    pub fn insert(&mut self, index: usize, ch: char) -> JsValue {
        let op = self.inner.insert_local(index, ch);
        serde_wasm_bindgen::to_value(&op).expect("Operation serialization cannot fail")
    }

    pub fn delete(&mut self, index: usize) -> JsValue {
        let op = self.inner.delete_local(index);
        serde_wasm_bindgen::to_value(&op).expect("Operation serialization cannot fail")
    }

    #[wasm_bindgen(js_name = applyRemote)]
    pub fn apply_remote(&mut self, op: JsValue) -> Result<(), JsValue> {
        let op: Operation = serde_wasm_bindgen::from_value(op)
            .map_err(|e| JsValue::from_str(&format!("invalid operation: {e}")))?;
        self.inner.apply_remote(op);
        Ok(())
    }

    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[wasm_bindgen(js_name = isEmpty)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[wasm_bindgen(js_name = siteId)]
    pub fn site_id(&self) -> u32 {
        self.inner.site_id()
    }
}
