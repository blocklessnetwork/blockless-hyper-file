use crate::error::ParseError;

const HEADER_PREFIX: &'static [u8] = b"bytes=";

#[derive(Debug, PartialEq)]
pub struct HttpRange {
    pub start: u64,
    pub length: u64,
}
type Result<T> = std::result::Result<T, ParseError>;

impl HttpRange {
    pub fn parse(header: &str, file_size: u64) -> Result<Vec<HttpRange>> {
        Self::parse_bytes(header.as_bytes(), file_size)
    }

    fn parse_bytes(header: &[u8], file_size: u64) -> Result<Vec<HttpRange>> {
        if header.is_empty() {
            return Err(ParseError::InvalidRange);
        }
        if !header.starts_with(HEADER_PREFIX) {
            return Err(ParseError::InvalidRange);
        }
        let mut no_overlap = false;
        let ranges = header[HEADER_PREFIX.len()..]
            .split(|n| *n == b',')
            .filter_map(|srange| -> Option<Result<HttpRange>> {
                let srange = srange.trim();
                match Self::parse_single_range(srange, file_size) {
                    Ok(Some(o)) => Some(Ok(o)),
                    Ok(None) => {
                        no_overlap = true;
                        None
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<Vec<HttpRange>>>()?;
        if no_overlap && ranges.len() == 0 {
            return Err(ParseError::NoOverlap);
        }
        Ok(ranges)
    }

    fn parse_single_range(range: &[u8], file_size: u64) -> Result<Option<HttpRange>> {
        let mut split_range = range.splitn(2, |s| *s == b'-');
        let range_start = split_range.next().ok_or(ParseError::InvalidRange)?;
        let range_end = split_range.next().ok_or(ParseError::InvalidRange)?;
        if range_start.is_empty() {
            if range_end.is_empty() || range_end[0] == b'-' {
                return Err(ParseError::InvalidRange);
            }
            let mut length = range_end.to_u64().ok_or(ParseError::InvalidRange)?;
            if length == 0 {
                return Ok(None);
            }
            if length > file_size {
                length = file_size;
            }
            return Ok(Some(HttpRange {
                start: file_size - length,
                length,
            }));
        } else {
            let start = range_start.to_u64().ok_or(ParseError::InvalidRange)?;
            if start > file_size {
                return Ok(None);
            }
            let length = if range_end.is_empty() {
                file_size - start
            } else {
                let mut end = range_end.to_u64().ok_or(ParseError::InvalidRange)?;
                if start > end {
                    return Err(ParseError::InvalidRange);
                }
                if end >= file_size {
                    end = file_size - 1;
                }
                end - start + 1
            };
            Ok(Some(HttpRange { start, length }))
        }
    }
}

trait SliceEx {
    fn trim(&self) -> &Self;
    //parse the slice to u64 b"123" = 123
    fn to_u64(&self) -> Option<u64>;
}

impl SliceEx for [u8] {
    fn trim(&self) -> &Self {
        #[inline(always)]
        fn is_whitspace(b: &u8) -> bool {
            *b == b' ' || *b == b'\t'
        }

        #[inline(always)]
        fn is_not_whitspace(b: &u8) -> bool {
            !is_whitspace(b)
        }

        if let Some(left) = self.iter().position(is_not_whitspace) {
            if let Some(right) = self.iter().rposition(is_not_whitspace) {
                return &self[left..right + 1];
            } else {
                unreachable!("can't be happend.")
            }
        }
        &[]
    }

    fn to_u64(&self) -> Option<u64> {
        if self.is_empty() {
            return None;
        }
        let mut sum: u64 = 0;
        for v in self {
            if *v < b'0' || *v > b'9' {
                return None;
            }
            sum = sum.checked_mul(10)?.checked_add((*v - b'0') as _)?;
        }
        Some(sum)
    }
}

#[cfg(test)]
mod test {
    use super::ParseError;
    use super::*;

    macro_rules! test_error {
        ($parse: literal, $file_size: literal, $result: pat) => {
            let rs = HttpRange::parse($parse, $file_size);
            assert!(matches!(rs, $result));
        };
    }

    macro_rules! test_range {
        ($parse: literal, $file_size: literal, $result: expr) => {
            let rs = HttpRange::parse($parse, $file_size).unwrap();
            assert_eq!(rs, $result);
        };
    }

    #[test]
    fn test_parse() {
        test_error!("", 0, Err(ParseError::InvalidRange));
        test_error!("", 100, Err(ParseError::InvalidRange));
        test_range!(
            "bytes=-5",
            10,
            vec![HttpRange {
                start: 5,
                length: 5
            }]
        );
        test_range!(
            "bytes=0-5",
            10,
            vec![HttpRange {
                start: 0,
                length: 6
            }]
        );
        test_range!(
            "bytes=0-100",
            10,
            vec![HttpRange {
                start: 0,
                length: 10
            }]
        );
        test_range!(
            "bytes=0-",
            10,
            vec![HttpRange {
                start: 0,
                length: 10
            }]
        );
        test_range!(
            "bytes=   0- ",
            10,
            vec![HttpRange {
                start: 0,
                length: 10
            }]
        );
        test_range!(
            "bytes=   0-2 , 5-10",
            10,
            vec![
                HttpRange {
                    start: 0,
                    length: 3
                },
                HttpRange {
                    start: 5,
                    length: 5
                }
            ]
        );
        test_range!(
            "bytes=500-600,601-999",
            1000,
            vec![
                HttpRange {
                    start: 500,
                    length: 101
                },
                HttpRange {
                    start: 601,
                    length: 399
                }
            ]
        );
    }
}
