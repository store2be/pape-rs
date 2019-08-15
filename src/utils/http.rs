pub(crate) trait ReqwestResponseExt {
    fn filename(&self) -> Option<String>;
}

impl ReqwestResponseExt for reqwest::r#async::Response {
    fn filename(&self) -> Option<String> {
        use hyperx::header::Header;

        let value = self.headers().get("Content-Disposition")?;
        let value = hyperx::header::ContentDisposition::parse_header(&value).ok()?;
        let filename: Vec<u8> = value.parameters.into_iter().find_map(|param| match param {
            hyperx::header::DispositionParam::Filename(_charset, _lang_tag, filename) => {
                Some(filename)
            }
            _ => None,
        })?;
        String::from_utf8(filename).ok()
    }
}

pub(crate) fn extract_filename_from_uri(uri: &hyper::Uri) -> Option<&str> {
    match uri.path().split('/').last() {
        Some(name) if !name.is_empty() => Some(name),
        _ => None,
    }
}

/// For templates and assets. Stream an HTTP response's body directly to a file, without allocating
/// it all in program memory.
pub(crate) async fn client_response_body_to_file(
    mut response: reqwest::r#async::Response,
    path: std::path::PathBuf,
    size_limit: u32,
) -> Result<(), failure::Error> {
use futures::compat::*;
    use futures::stream::StreamExt;
    use tokio_fs::File;
    use tokio_io::AsyncWrite;

    let mut file = File::create(path).compat().await?;
    let mut body = response.body_mut().compat();

    // Running count of the downloaded file's size in bytes.
    let mut bytes_size: u32 = 0;

    while let Some(chunk) = body.next().await.transpose()? {
        bytes_size += chunk.len() as u32;
        if bytes_size > size_limit {
            return Err(failure::err_msg("File exceeded max asset size"));
        }

        futures01::future::poll_fn(|| file.poll_write(&chunk))
            .compat()
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod extract_filename_tests {
    use super::extract_filename_from_uri;
    use hyper::Uri;

    #[test]
    fn test_extract_filename_from_uri_works() {
        let assert_extracted = |input: &'static str, expected_output: Option<&'static str>| {
            let uri = input.parse::<Uri>().unwrap();
            assert_eq!(extract_filename_from_uri(&uri), expected_output);
        };

        assert_extracted("/logo.png", Some("logo.png"));
        assert_extracted("/assets/", None);
        assert_extracted("/assets/icon", Some("icon"));
        assert_extracted("/", None);
        assert_extracted("http://www.store2be.com", None);
    }
}
