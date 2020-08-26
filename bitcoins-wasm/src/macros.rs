//! Contains macros for use in this crate
//!
//! Some Notes on the macros:
//! - `wrap_struct` creates a new-type around a struct, and implements pass-throughs
//!   for serialization and deserialization
//! - `impl_simple_getter` creates a getter function for a pub property of a simple
//!   type. This works for any type natively supported by wasm_bindgen, e.g. u32.
//! - `impl_getter_passthrough` is equivalent to `impl_simple_getter` but wrapper
//!   getter functions instead of public properties.
//! - `impl_wrapped_getter` creates a getter function for public properties that are
//!   themselves structs that we have wrapped with `wrap_struct`. E.g. the TxIn's
//!   `outpoint` property.
//! - `impl_wrapped_getter_passthrough` creates a getter function for public getters
//!    that return structs that we have wrapped with `wrap_struct`. E.g. the `LegacyTx`
//!    class's `txid()` method;
//! - `impl_prefix_vec_access` generates getters and setters for prefix vecs

/// This macro wraps and implements a wrapper around the `Ser` trait
macro_rules! wrap_struct {
    (
        $(#[$outer:meta])*
        $module:ident::$name:ident
    ) => {
        $(#[$outer])*
        #[wasm_bindgen(inspectable)]
        #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
        pub struct $name($module::$name);

        impl $name {
            /// Return a clone of the underlying object.
            pub fn inner(&self) -> $module::$name {
                self.0.clone()
            }
        }

        impl From<$module::$name> for $name {
            fn from(f: $module::$name) -> Self {
                Self(f)
            }
        }

        impl From<&$module::$name> for $name {
            fn from(f: &$module::$name) -> Self {
                Self(f.clone())
            }
        }

        impl From<$name> for $module::$name {
            fn from(f: $name) -> Self {
                f.0
            }
        }

        #[wasm_bindgen]
        impl $name {
            /// Deserialize from a `Uint8Array`
            #[allow(clippy::useless_asref)]
            pub fn read_from(buf: &[u8]) -> Result<$name, JsValue> {
                $module::$name::read_from(&mut buf.as_ref())
                    .map(Self::from)
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)
            }

            /// Serialize to a `Uint8Array`
            pub fn write_bytes(&self) -> Result<js_sys::Uint8Array, JsValue> {
                let mut v = vec![];
                self.0.write_to(&mut v)
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)?;
                Ok(js_sys::Uint8Array::from(&v[..]))
            }

            /// Deserialize from hex.
            pub fn deserialize_hex(s: String) -> Result<$name, JsValue> {
                $module::$name::deserialize_hex(&s)
                    .map(Self::from)
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)
            }

            /// Serialize to a hex string.
            pub fn serialize_hex(&self) -> String {
                self.0.serialize_hex()
            }

            /// Deserialize from base64.
            pub fn deserialize_base64(s: String) -> Result<$name, JsValue> {
                $module::$name::deserialize_base64(&s)
                    .map(Self::from)
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)
            }

            /// Serialize to a base64 string.
            pub fn serialize_base64(&self) -> String {
                self.0.serialize_base64()
            }
        }
    }
}

/// Implements a getter
#[macro_export]
macro_rules! impl_simple_getter {
    ($class:ident, $prop:ident, $type:ty) => {
        #[wasm_bindgen]
        impl $class {
            /// A simple property getter
            #[wasm_bindgen(method, getter)]
            pub fn $prop(&self) -> $type {
                (self.0).$prop
            }
        }
    };
}

/// Implements a getter calling an underlying getter function
#[macro_export]
macro_rules! impl_getter_passthrough {
    ($class:ident, $prop:ident, $type:ty) => {
        #[wasm_bindgen]
        impl $class {
            /// A passthrough getter for a calculated property
            #[wasm_bindgen(method, getter)]
            pub fn $prop(&self) -> $type {
                (self.0).$prop()
            }
        }
    };
}

/// Implements a getter that returns a wasm-wrapped struct
#[macro_export]
macro_rules! impl_wrapped_getter {
    ($class:ident, $prop:ident, $type:ty) => {
        #[wasm_bindgen]
        impl $class {
            /// A property getter that wraps the result in a JS-friendly object
            #[wasm_bindgen(method, getter)]
            pub fn $prop(&self) -> $type {
                (self.0).$prop.into()
            }
        }
    };
}

/// Implements a getter that calls an underlying getter function and returns a wasm-wrapped struct
#[macro_export]
macro_rules! impl_wrapped_getter_passthrough {
    ($class:ident, $prop:ident, $type:ty) => {
        #[wasm_bindgen]
        impl $class {
            /// A calculated property getter that wraps the result in a JS-friendly object
            #[wasm_bindgen(method, getter)]
            pub fn $prop(&self) -> $type {
                (self.0).$prop().into()
            }
        }
    };
}

macro_rules! impl_prefix_vec_access {
    ($module:ident::$class:ident, $inner_module:ident::$inner_class:ident) => {
        #[wasm_bindgen]
        impl $class {
            #[wasm_bindgen(constructor)]
            /// Simple JS constructor. Passes through to `null`
            pub fn new() -> $class {
                Self::null()
            }

            /// Instantiate an empty prefixed vector
            pub fn null() -> $class {
                $class($module::$class::default())
            }

            #[wasm_bindgen(method, getter)]
            /// Get the length of the vector
            pub fn length(&self) -> usize {
                self.0.len()
            }

            #[wasm_bindgen(method, getter)]
            /// Get the length of the CompactInt prefix
            /// Determine the byte-length of the vector length prefix
            pub fn len_prefix(&self) -> u8 {
                coins_core::ser::prefix_byte_len(self.length() as u64)
            }

            /// Push input to the vector
            pub fn push(&mut self, input: &$inner_class) {
                self.0.push(input.0.clone())
            }

            /// Get the element at `index`
            pub fn get(&self, index: usize) -> $inner_class {
                self.0[index].clone().into()
            }

            /// Overwrite the element at `index`
            pub fn set(&mut self, index: usize, item: &$inner_class) {
                self.0[index] = item.clone().into()
            }

            #[wasm_bindgen(method, getter)]
            /// Return an array containing clones of the underlying items
            pub fn items(&self) -> js_sys::Array {
                self.0
                    .iter()
                    .map(Clone::clone)
                    .map($inner_class::from)
                    .map(JsValue::from)
                    .collect()
            }
        }
    };
}

macro_rules! impl_builders {
    ($builder:ident, $enc:ident) => {
        /// This is a generic builder for Bitcoin transactions. It allows you to easily build legacy and
        /// witness transactions.
        ///
        /// Note: due to Bitcoin consensus rules, the order of inputs and outputs may be semantically
        /// meaningful. E.g. when signing a transaction with the `SINGLE` sighash mode.
        ///
        /// It is parameterized with an address encoder, so that the same struct and logic can be used on
        /// mainnet and testnet.
        #[wasm_bindgen(inspectable)]
        #[derive(Debug, Clone)]
        pub struct $builder(bitcoins::builder::BitcoinTxBuilder<bitcoins::enc::$enc>);

        impl From<bitcoins::builder::BitcoinTxBuilder<bitcoins::enc::$enc>> for $builder {
            fn from(f: bitcoins::builder::BitcoinTxBuilder<bitcoins::enc::$enc>) -> Self {
                Self(f)
            }
        }

        impl Default for $builder {
            fn default() -> $builder {
                $builder::new()
            }
        }

        #[wasm_bindgen]
        impl $builder {
            #[wasm_bindgen(constructor)]
            /// Instantiate a new builder
            pub fn new() -> $builder {
                bitcoins::builder::BitcoinTxBuilder::new().into()
            }

            /// Instantate a builder from a tx
            pub fn from_tx(tx: &crate::types::tx::BitcoinTx) -> $builder {
                bitcoins::builder::BitcoinTxBuilder::from_tx_ref(&tx.inner()).into()
            }

            /// Instantate a builder from a hex-encoded tx
            pub fn from_hex_tx(hex: String) -> Result<$builder, JsValue> {
                bitcoins::builder::BitcoinTxBuilder::from_hex_tx(&hex)
                    .map(Into::into)
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)
            }

            /// Set the builder version
            pub fn version(self, version: u32) -> $builder {
                self.0.version(version).into()
            }

            /// Spend an outpoint
            pub fn spend(self, outpoint: BitcoinOutpoint, sequence: u32) -> $builder {
                self.0.spend(outpoint, sequence).into()
            }

            /// Pay an address
            pub fn pay(self, value: u64, address: &str) -> Result<$builder, JsValue> {
                let addr = bitcoins::enc::$enc::string_to_address(address)
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)?;
                self.0
                    .pay(value, &addr)
                    .map($builder::from)
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)
            }

            /// Extend the vin with several inputs
            pub fn extend_inputs(self, inputs: Vin) -> $builder {
                self.0
                    .extend_inputs(bitcoins::types::txin::Vin::from(inputs))
                    .into()
            }

            /// Extend the vout with several outputs
            pub fn extend_outputs(self, outputs: Vout) -> $builder {
                self.0
                    .extend_outputs(bitcoins::types::txout::Vout::from(outputs))
                    .into()
            }

            /// Set the locktime
            pub fn locktime(self, locktime: u32) -> $builder {
                self.0.locktime(locktime).into()
            }

            /// Add witnesses and implicitly convert to a witness builder.
            pub fn extend_witnesses(self, witnesses: JsValue) -> $builder {
                self.0
                    .extend_witnesses(witnesses.into_serde().unwrap())
                    .into()
            }

            /// Consume the builder and produce a transaction
            pub fn build(self) -> Result<crate::types::tx::BitcoinTx, JsValue> {
                self.0
                    .build()
                    .map(crate::types::tx::BitcoinTx::from)
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)
            }
        }
    };
}

macro_rules! impl_encoder {
    (
        $(#[$outer:meta])*
        $enc_name:ident
    ) => {
        $(#[$outer])*
        #[wasm_bindgen]
        pub struct $enc_name;

        #[wasm_bindgen]
        impl $enc_name {
            /// Attempt to encode a `RecipientIdentifier` as an `Address`.
            pub fn encode_address(s: &[u8]) -> Result<Address, JsValue> {
                bitcoins::enc::$enc_name::encode_address(&bitcoins::types::script::ScriptPubkey::from(s))
                    .map(Address::from)
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)
            }

            /// Attempt to decode a `RecipientIdentifier` from an `Address`.
            pub fn decode_address(addr: Address) -> Result<js_sys::Uint8Array, JsValue> {
                let decoded = bitcoins::enc::$enc_name::decode_address(&addr.into())
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)?;
                Ok(js_sys::Uint8Array::from(decoded.items()))
            }

            /// Attempt to convert a string into an `Address`.
            pub fn string_to_address(s: &str) -> Result<Address, JsValue> {
                bitcoins::enc::$enc_name::string_to_address(s)
                    .map(Address::from)
                    .map_err(crate::types::errors::WasmError::from)
                    .map_err(JsValue::from)
            }
        }
    }
}

macro_rules! impl_network {
    (
        $(#[$outer:meta])*
        $network_name:ident, $builder_name:ident, $encoder_name:ident
    ) => {
        /// A Network object. This
        #[wasm_bindgen(inspectable)]
        #[derive(Debug)]
        pub struct $network_name;

        #[wasm_bindgen]
        impl $network_name {
            /// Return a new transaction builder for this network.
            pub fn tx_builder() -> $builder_name {
                $builder_name::new()
            }

            /// Instantiate a transaction builder from a hex-serialized transaction
            /// Throws if the hex-string is not a properly-formatted bitcoin transaction
            pub fn builder_from_hex(hex_tx: String) -> Result<$builder_name, JsValue> {
                $builder_name::from_hex_tx(hex_tx)
            }

            /// Encode a Uint8Array as an address with this network's version info.
            /// Throws for non-standard scripts
            pub fn encode_address(s: &[u8]) -> Result<Address, JsValue> {
                $encoder_name::encode_address(s)
            }

            /// Attempt to decode a `RecipientIdentifier` from an `Address`.
            /// Throws if the detected version info does not match this network.
            pub fn decode_address(addr: Address) -> Result<js_sys::Uint8Array, JsValue> {
                $encoder_name::decode_address(addr)
            }

            /// Attempt to convert a string into an `Address`.
            /// Throws if the string is not an address for this network.
            pub fn string_to_address(s: &str) -> Result<Address, JsValue> {
                $encoder_name::string_to_address(s)
            }
        }
    };
}
