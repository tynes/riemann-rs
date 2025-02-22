use std::marker::PhantomData;

use coins_core::hashes::{Digest, Hash256};

use crate::{
    curve::model::{PointDeserialize, ScalarDeserialize, Secp256k1Backend},
    keys::{GenericPrivkey, GenericPubkey},
    model::{HasPrivkey, HasPubkey, XKey},
    primitives::{ChainCode, Hint, KeyFingerprint, XKeyInfo},
    xkeys::{GenericXPriv, GenericXPub},
    Bip32Error,
};

/// Decode a bytevector from a base58 check string
pub fn decode_b58_check(s: &str) -> Result<Vec<u8>, Bip32Error> {
    let data: Vec<u8> = bs58::decode(s).into_vec()?;
    let idx = data.len() - 4;
    let payload = &data[..idx];
    let checksum = &data[idx..];

    let digest = &Hash256::digest(payload);

    let mut expected = [0u8; 4];
    expected.copy_from_slice(&digest.as_slice()[..4]);
    if expected != checksum {
        Err(Bip32Error::BadB58Checksum)
    } else {
        Ok(payload.to_vec())
    }
}

/// Encode a vec into a base58 check String
pub fn encode_b58_check(v: &[u8]) -> String {
    let digest = &Hash256::digest(v);

    let mut checksum = [0u8; 4];
    checksum.copy_from_slice(&digest.as_slice()[..4]);

    let mut data = v.to_vec();
    data.extend(&checksum);

    bs58::encode(data).into_string()
}

/// Contains network-specific serialization information
pub trait NetworkParams {
    /// The Bip32 privkey version bytes
    const PRIV_VERSION: u32;
    /// The Bip49 privkey version bytes
    const BIP49_PRIV_VERSION: u32;
    /// The Bip84 pubkey version bytes
    const BIP84_PRIV_VERSION: u32;
    /// The Bip32 pubkey version bytes
    const PUB_VERSION: u32;
    /// The Bip49 pubkey version bytes
    const BIP49_PUB_VERSION: u32;
    /// The Bip84 pubkey version bytes
    const BIP84_PUB_VERSION: u32;
}

/// Bip32/49/84 encoder
pub trait XKeyEncoder {
    #[doc(hidden)]
    fn write_key_details<K, W>(writer: &mut W, key: &K) -> Result<usize, Bip32Error>
    where
        K: XKey,
        W: std::io::Write,
    {
        let mut written = writer.write(&[key.depth()])?;
        written += writer.write(&key.parent().0)?;
        written += writer.write(&key.index().to_be_bytes())?;
        written += writer.write(&key.chain_code().0)?;
        Ok(written)
    }

    /// Serialize the xpub to `std::io::Write`
    fn write_xpub<'a, W, T>(writer: &mut W, key: &GenericXPub<'a, T>) -> Result<usize, Bip32Error>
    where
        W: std::io::Write,
        T: Secp256k1Backend;

    /// Serialize the xpriv to `std::io::Write`
    fn write_xpriv<'a, W, T>(
        writer: &mut W,
        key: &GenericXPriv<'a, T>,
    ) -> Result<usize, Bip32Error>
    where
        W: std::io::Write,
        T: Secp256k1Backend;

    #[doc(hidden)]
    fn read_depth<R>(reader: &mut R) -> Result<u8, Bip32Error>
    where
        R: std::io::Read,
    {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    #[doc(hidden)]
    fn read_parent<R>(reader: &mut R) -> Result<KeyFingerprint, Bip32Error>
    where
        R: std::io::Read,
    {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(buf.into())
    }

    #[doc(hidden)]
    fn read_index<R>(reader: &mut R) -> Result<u32, Bip32Error>
    where
        R: std::io::Read,
    {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    #[doc(hidden)]
    fn read_chain_code<R>(reader: &mut R) -> Result<ChainCode, Bip32Error>
    where
        R: std::io::Read,
    {
        let mut buf = [0u8; 32];
        reader.read_exact(&mut buf)?;
        Ok(buf.into())
    }

    #[doc(hidden)]
    fn read_xpriv_body<'a, R, T>(
        reader: &mut R,
        hint: Hint,
        backend: Option<&'a T>,
    ) -> Result<GenericXPriv<'a, T>, Bip32Error>
    where
        R: std::io::Read,
        T: Secp256k1Backend,
    {
        let depth = Self::read_depth(reader)?;
        let parent = Self::read_parent(reader)?;
        let index = Self::read_index(reader)?;
        let chain_code = Self::read_chain_code(reader)?;

        let mut buf = [0u8];
        reader.read_exact(&mut buf)?;
        if buf != [0] {
            return Err(Bip32Error::BadPadding(buf[0]));
        }

        let mut buf = [0u8; 32];
        reader.read_exact(&mut buf)?;
        let key = T::Privkey::from_privkey_array(buf)?;

        Ok(GenericXPriv {
            info: XKeyInfo {
                depth,
                parent,
                index,
                chain_code,
                hint,
            },
            privkey: GenericPrivkey { key, backend },
        })
    }

    #[doc(hidden)]
    // Can be used for unhealthy but sometimes-desiable behavior. E.g. accepting an xpriv from any
    // network.
    fn read_xpriv_without_network<'a, R, T>(
        reader: &mut R,
        backend: Option<&'a T>,
    ) -> Result<GenericXPriv<'a, T>, Bip32Error>
    where
        R: std::io::Read,
        T: Secp256k1Backend,
    {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;

        Self::read_xpriv_body(reader, Hint::Legacy, backend)
    }

    /// Attempt to instantiate an `XPriv` from a `std::io::Read`
    ///
    /// # Note
    ///
    /// If passing in None, you must hint the return type. This can be the convenience type alias
    /// (e.g. XPub) or the full type `GenericXPub<coins_bip32::backends::curve::Secp256k1>`
    ///
    /// ```
    /// use coins_bip32::{Bip32Error, XPriv, enc::{XKeyEncoder, MainnetEncoder}};
    /// # fn main() -> Result<(), Bip32Error> {
    /// let xpriv_str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi".to_owned();
    ///
    /// // // can't infer type of generic parameter
    /// // let xpriv = MainnetEncoder::xpriv_from_base58(&xpriv_str, None)?;
    ///
    /// let xpriv: XPriv = MainnetEncoder::xpriv_from_base58(&xpriv_str, None)?;
    /// # Ok(())
    /// # }
    /// ```
    fn read_xpriv<'a, R, T>(
        reader: &mut R,
        backend: Option<&'a T>,
    ) -> Result<GenericXPriv<'a, T>, Bip32Error>
    where
        R: std::io::Read,
        T: Secp256k1Backend;

    #[doc(hidden)]
    fn read_xpub_body<'a, R, T>(
        reader: &mut R,
        hint: Hint,
        backend: Option<&'a T>,
    ) -> Result<GenericXPub<'a, T>, Bip32Error>
    where
        R: std::io::Read,
        T: Secp256k1Backend,
    {
        let depth = Self::read_depth(reader)?;
        let parent = Self::read_parent(reader)?;
        let index = Self::read_index(reader)?;
        let chain_code = Self::read_chain_code(reader)?;

        let mut buf = [0u8; 33];
        reader.read_exact(&mut buf)?;
        let key = T::Pubkey::from_pubkey_array(buf)?;

        Ok(GenericXPub {
            info: XKeyInfo {
                depth,
                parent,
                index,
                chain_code,
                hint,
            },
            pubkey: GenericPubkey { key, backend },
        })
    }

    #[doc(hidden)]
    // Can be used for unhealthy but sometimes-desiable behavior. E.g. accepting an xpub from any
    // network.
    fn read_xpub_without_network<'a, R, T>(
        reader: &mut R,
        backend: Option<&'a T>,
    ) -> Result<GenericXPub<'a, T>, Bip32Error>
    where
        R: std::io::Read,
        T: Secp256k1Backend,
    {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;

        Self::read_xpub_body(reader, Hint::Legacy, backend)
    }

    /// Attempt to instantiate an `XPriv` from a `std::io::Read`
    ///
    /// # Note
    ///
    /// If passing in None, you must hint the return type. This can be the convenience type alias
    /// (e.g. XPub) or the full type `GenericXPub<coins_bip32::backends::curve::Secp256k1>`
    ///
    /// ```
    /// use coins_bip32::{Bip32Error, XPub, enc::{XKeyEncoder, MainnetEncoder}};
    /// # fn main() -> Result<(), Bip32Error> {
    /// let xpub_str = "xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y".to_owned();
    ///
    /// // // can't infer type of generic parameter
    /// // let xpub = MainnetEncoder::xpub_from_base58(&xpub_str, None)?;
    ///
    /// let xpub: XPub = MainnetEncoder::xpub_from_base58(&xpub_str, None)?;
    /// # Ok(())
    /// # }
    /// ```
    fn read_xpub<'a, R, T>(
        reader: &mut R,
        backend: Option<&'a T>,
    ) -> Result<GenericXPub<'a, T>, Bip32Error>
    where
        R: std::io::Read,
        T: Secp256k1Backend;

    /// Serialize an XPriv to base58
    fn xpriv_to_base58<'a, T>(k: &GenericXPriv<'a, T>) -> Result<String, Bip32Error>
    where
        T: Secp256k1Backend,
    {
        let mut v: Vec<u8> = vec![];
        Self::write_xpriv(&mut v, k)?;
        Ok(encode_b58_check(&v))
    }

    /// Serialize an XPub to base58
    fn xpub_to_base58<'a, T>(k: &GenericXPub<'a, T>) -> Result<String, Bip32Error>
    where
        T: Secp256k1Backend,
    {
        let mut v: Vec<u8> = vec![];
        Self::write_xpub(&mut v, k)?;
        Ok(encode_b58_check(&v))
    }

    /// Attempt to read an XPriv from a b58check string.
    ///
    /// # Note
    ///
    /// If passing in None, you must hint the return type. This can be the convenience type alias
    /// (e.g. XPub) or the full type `GenericXPub<coins_bip32::backends::curve::Secp256k1>`
    ///
    /// ```
    /// use coins_bip32::{Bip32Error, XPriv, enc::{XKeyEncoder, MainnetEncoder}};
    /// # fn main() -> Result<(), Bip32Error> {
    /// let xpriv_str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi".to_owned();
    ///
    /// // // can't infer type of generic parameter
    /// // let xpriv = MainnetEncoder::xpriv_from_base58(&xpriv_str, None)?;
    ///
    /// let xpriv: XPriv = MainnetEncoder::xpriv_from_base58(&xpriv_str, None)?;
    /// # Ok(())
    /// # }
    /// ```
    fn xpriv_from_base58<'a, T>(
        s: &str,
        backend: Option<&'a T>,
    ) -> Result<GenericXPriv<'a, T>, Bip32Error>
    where
        T: Secp256k1Backend,
    {
        let data = decode_b58_check(s)?;
        Self::read_xpriv(&mut &data[..], backend)
    }

    /// Attempt to read an XPub from a b58check string
    ///
    /// # Note
    ///
    /// If passing in None, you must hint the return type. This can be the convenience type alias
    /// (e.g. XPub) or the full type `GenericXPub<coins_bip32::backends::curve::Secp256k1>`
    ///
    /// ```
    /// use coins_bip32::{Bip32Error, XPub, enc::{XKeyEncoder, MainnetEncoder}};
    /// # fn main() -> Result<(), Bip32Error> {
    /// let xpub_str = "xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y".to_owned();
    ///
    /// // // can't infer type of generic parameter
    /// // let xpub = MainnetEncoder::xpub_from_base58(&xpub_str, None)?;
    ///
    /// let xpub: XPub = MainnetEncoder::xpub_from_base58(&xpub_str, None)?;
    /// # Ok(())
    /// # }
    /// ```
    fn xpub_from_base58<'a, T>(
        s: &str,
        backend: Option<&'a T>,
    ) -> Result<GenericXPub<'a, T>, Bip32Error>
    where
        T: Secp256k1Backend,
    {
        let data = decode_b58_check(s)?;
        Self::read_xpub(&mut &data[..], backend)
    }
}

params!(
    /// Mainnet encoding param
    Main {
        bip32: 0x0488_ADE4,
        bip49: 0x049d_7878,
        bip84: 0x04b2_430c,
        bip32_pub: 0x0488_B21E,
        bip49_pub: 0x049d_7cb2,
        bip84_pub: 0x04b2_4746
    }
);

params!(
    /// Testnet encoding param
    Test {
        bip32: 0x0435_8394,
        bip49: 0x044a_4e28,
        bip84: 0x045f_18bc,
        bip32_pub: 0x0435_87CF,
        bip49_pub: 0x044a_5262,
        bip84_pub: 0x045f_1cf6
    }
);

/// Parameterizable Bitcoin encoder
#[derive(Debug, Clone)]
pub struct BitcoinEncoder<P: NetworkParams>(PhantomData<fn(P) -> P>);

impl<P: NetworkParams> XKeyEncoder for BitcoinEncoder<P> {
    /// Serialize the xpub to `std::io::Write`
    fn write_xpub<'a, W, T>(writer: &mut W, key: &GenericXPub<'a, T>) -> Result<usize, Bip32Error>
    where
        W: std::io::Write,
        T: Secp256k1Backend,
    {
        let version = match key.hint() {
            Hint::Legacy => P::PUB_VERSION,
            Hint::Compatibility => P::BIP49_PUB_VERSION,
            Hint::SegWit => P::BIP84_PUB_VERSION,
        };
        let mut written = writer.write(&version.to_be_bytes())?;
        written += Self::write_key_details(writer, key)?;
        written += writer.write(&key.pubkey_bytes())?;
        Ok(written)
    }

    /// Serialize the xpriv to `std::io::Write`
    fn write_xpriv<'a, W, T>(writer: &mut W, key: &GenericXPriv<'a, T>) -> Result<usize, Bip32Error>
    where
        W: std::io::Write,
        T: Secp256k1Backend,
    {
        let version = match key.hint() {
            Hint::Legacy => P::PRIV_VERSION,
            Hint::Compatibility => P::BIP49_PRIV_VERSION,
            Hint::SegWit => P::BIP84_PRIV_VERSION,
        };
        let mut written = writer.write(&version.to_be_bytes())?;
        written += Self::write_key_details(writer, key)?;
        written += writer.write(&[0])?;
        written += writer.write(&key.privkey_bytes())?;
        Ok(written)
    }

    fn read_xpriv<'a, R, T>(
        reader: &mut R,
        backend: Option<&'a T>,
    ) -> Result<GenericXPriv<'a, T>, Bip32Error>
    where
        R: std::io::Read,
        T: Secp256k1Backend,
    {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        let version_bytes = u32::from_be_bytes(buf);

        // Can't use associated constants in matches :()
        let hint = if version_bytes == P::PRIV_VERSION {
            Hint::Legacy
        } else if version_bytes == P::BIP49_PRIV_VERSION {
            Hint::Compatibility
        } else if version_bytes == P::BIP84_PRIV_VERSION {
            Hint::SegWit
        } else {
            return Err(Bip32Error::BadXPrivVersionBytes(buf));
        };
        Self::read_xpriv_body(reader, hint, backend)
    }

    fn read_xpub<'a, R, T>(
        reader: &mut R,
        backend: Option<&'a T>,
    ) -> Result<GenericXPub<'a, T>, Bip32Error>
    where
        R: std::io::Read,
        T: Secp256k1Backend,
    {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        let version_bytes = u32::from_be_bytes(buf);

        // Can't use associated constants in matches :()
        let hint = if version_bytes == P::PUB_VERSION {
            Hint::Legacy
        } else if version_bytes == P::BIP49_PUB_VERSION {
            Hint::Compatibility
        } else if version_bytes == P::BIP84_PUB_VERSION {
            Hint::SegWit
        } else {
            return Err(Bip32Error::BadXPrivVersionBytes(buf));
        };
        Self::read_xpub_body(reader, hint, backend)
    }
}

/// XKeyEncoder for Mainnet xkeys
pub type MainnetEncoder = BitcoinEncoder<Main>;
/// XKeyEncoder for Testnet xkeys
pub type TestnetEncoder = BitcoinEncoder<Test>;

#[cfg(test)]
mod test {
    use super::*;
    use crate::xkeys::XPriv;

    #[test]
    fn it_can_read_keys_without_a_backend() {
        let xpriv_str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi".to_owned();
        let _xpriv: XPriv = MainnetEncoder::xpriv_from_base58(&xpriv_str, None).unwrap();
    }
}
