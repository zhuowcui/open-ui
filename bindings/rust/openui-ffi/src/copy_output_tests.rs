use super::*;

#[test]
fn invalid_copy_storage_preserves_count_and_destination() {
    let mut count = 37;
    for bytes in [b"owned".as_slice(), b"".as_slice()] {
        let error = copy_bytes_to_c(bytes, ptr::null_mut(), 1, &mut count, "bytes").unwrap_err();
        assert_eq!(error.status, OuiStatus::InvalidArgument);
        assert_eq!(count, 37);
    }
    for values in [&[2u64, 5][..], &[][..]] {
        let error = copy_array_to_c(values, ptr::null_mut(), 1, &mut count, "values").unwrap_err();
        assert_eq!(error.status, OuiStatus::InvalidArgument);
        assert_eq!(count, 37);
    }
    let mut bytes = [0xa5; 8];
    assert_eq!(
        copy_bytes_to_c(
            b"owned",
            bytes.as_mut_ptr(),
            bytes.len(),
            ptr::null_mut(),
            "bytes"
        )
        .unwrap_err()
        .status,
        OuiStatus::InvalidArgument
    );
    assert_eq!(bytes, [0xa5; 8]);
    let mut values = [0xa5u64; 4];
    assert_eq!(
        copy_array_to_c(
            &[2, 5],
            values.as_mut_ptr(),
            values.len(),
            ptr::null_mut(),
            "values"
        )
        .unwrap_err()
        .status,
        OuiStatus::InvalidArgument
    );
    assert_eq!(values, [0xa5; 4]);
}

#[test]
fn copy_queries_and_short_buffers_report_owned_lengths_without_partial_writes() {
    let mut count = 0;
    copy_bytes_to_c(b"owned", ptr::null_mut(), 0, &mut count, "bytes").unwrap();
    assert_eq!(count, 5);
    let mut bytes = [0xa5; 8];
    assert_eq!(
        copy_bytes_to_c(b"owned", bytes.as_mut_ptr(), 2, &mut count, "bytes")
            .unwrap_err()
            .status,
        OuiStatus::BufferTooSmall
    );
    assert_eq!(count, 5);
    assert_eq!(bytes, [0xa5; 8]);
    copy_bytes_to_c(b"owned", bytes.as_mut_ptr(), 8, &mut count, "bytes").unwrap();
    assert_eq!(&bytes[..5], b"owned");
    assert_eq!(&bytes[5..], &[0xa5; 3]);

    copy_array_to_c(&[2u64, 5], ptr::null_mut(), 0, &mut count, "values").unwrap();
    assert_eq!(count, 2);
    let mut values = [0xa5u64; 4];
    assert_eq!(
        copy_array_to_c(&[2, 5], values.as_mut_ptr(), 1, &mut count, "values")
            .unwrap_err()
            .status,
        OuiStatus::BufferTooSmall
    );
    assert_eq!(count, 2);
    assert_eq!(values, [0xa5; 4]);
    copy_array_to_c(&[2, 5], values.as_mut_ptr(), 4, &mut count, "values").unwrap();
    assert_eq!(values, [2, 5, 0xa5, 0xa5]);
}
