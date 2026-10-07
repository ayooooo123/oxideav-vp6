//! The registry decoder reports the size of the frame it last returned
//! (oxideav-core `Decoder::output_video_dimensions` /
//! `output_pixel_format`), including a size change at a keyframe. The
//! keyframes come from the crate's own intra encoder.

use oxideav_core::{CodecId, Decoder, Frame, Packet, PixelFormat, TimeBase};
use oxideav_vp6::decoder::Vp6CodecDecoder;
use oxideav_vp6::intra_encode::encode_intra_frame;

/// A keyframe of `hf`×`vf` fragments (8×8 luma samples each).
fn keyframe(hf: usize, vf: usize) -> Vec<u8> {
    let mut frame = oxideav_vp6::Frame::new(hf, vf);
    for plane in [&mut frame.y, &mut frame.u, &mut frame.v] {
        for (i, s) in plane.samples_mut().iter_mut().enumerate() {
            *s = (i * 7 % 256) as u8;
        }
    }
    encode_intra_frame(&frame, 40).expect("encode")
}

#[test]
fn each_frame_reports_its_own_size() {
    let frames = [(keyframe(4, 4), (32, 32)), (keyframe(6, 2), (48, 16))];
    let mut dec = Vp6CodecDecoder::new(CodecId::new("vp6"));
    let mut last = None;
    for (at, (data, (w, h))) in frames.into_iter().enumerate() {
        dec.send_packet(&Packet::new(0, TimeBase::new(1, 25), data))
            .expect("send");
        if last.is_some() {
            assert_eq!(
                dec.output_video_dimensions(),
                last,
                "frame {at}: before it is returned"
            );
        }
        let frame = match dec.receive_frame() {
            Ok(Frame::Video(frame)) => frame,
            other => panic!("frame {at}: {other:?}"),
        };
        assert_eq!(dec.output_video_dimensions(), Some((w, h)), "frame {at}");
        assert_eq!(
            dec.output_pixel_format(),
            Some(PixelFormat::Yuv420P),
            "frame {at}"
        );
        let planes = frame.image_planes();
        assert_eq!(planes.len(), 3, "frame {at}: planes");
        for (i, plane) in planes.iter().enumerate() {
            let (pw, ph) = if i == 0 { (w, h) } else { (w / 2, h / 2) };
            assert_eq!(plane.stride, pw as usize, "frame {at}: plane {i} stride");
            assert_eq!(
                plane.data.len(),
                (pw * ph) as usize,
                "frame {at}: plane {i} rows"
            );
        }
        last = Some((w, h));
    }
}
