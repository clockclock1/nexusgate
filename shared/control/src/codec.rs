use bytes::{BufMut, BytesMut};
use p2p_protocol::{encode_message, try_decode_message, ControlMessage, MAX_FRAME_SIZE};
use tokio_util::codec::{Decoder, Encoder};

/// Tokio codec: 4-byte big-endian length + JSON ControlMessage.
#[derive(Debug, Default, Clone)]
pub struct ControlCodec;

impl Decoder for ControlCodec {
    type Item = ControlMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match try_decode_message(src) {
            Ok(v) => Ok(v),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
        }
    }
}

impl Encoder<ControlMessage> for ControlCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: ControlMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let encoded = encode_message(&item)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        if encoded.len() > 4 + MAX_FRAME_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "frame too large",
            ));
        }
        dst.reserve(encoded.len());
        dst.put_slice(&encoded);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2p_protocol::ControlMessage;

    #[test]
    fn codec_roundtrip() {
        let mut codec = ControlCodec;
        let msg = ControlMessage::heartbeat(7);
        let mut buf = BytesMut::new();
        codec.encode(msg.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(decoded.message_type(), "HEARTBEAT");
    }
}
