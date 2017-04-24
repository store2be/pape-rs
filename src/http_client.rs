use error::Error;
use futures::future;
use futures::{Future, Stream};
use hyper;
use hyper::client::{Client, Response};
use tokio_core::reactor::Handle;
use hyper::header::{Location};
use hyper::{Uri, StatusCode};

enum GetResult {
    Ok(Response),
    Redirect(Uri),
}

fn determine_get_result(res: Response) -> Result<GetResult, Error> {
    match res.status() {
        StatusCode::TemporaryRedirect | StatusCode::PermanentRedirect => {
            match res.headers().get::<Location>() {
                Some(location) => Ok(GetResult::Redirect(location.parse()?)),
                None => Err(Error::UnprocessableEntity),
            }
        },
        StatusCode::Ok => Ok(GetResult::Ok(res)),
        _ => Err(Error::UnprocessableEntity),
    }
}

pub fn download_file(handle: &Handle, uri: Uri) -> Box<Future<Item=Vec<u8>, Error=Error>>
{
    // loop_fn is for tail-recursive futures. See:
    // https://docs.rs/futures/0.1.9/futures/future/fn.loop_fn.html
    let client = Client::new(handle);
    Box::new(future::loop_fn(uri, move |uri| {
        client.get(uri)
            .map_err(Error::from)
            .and_then(|res| {
                match determine_get_result(res) {
                    Ok(GetResult::Redirect(redirect_uri)) => {
                        Ok(future::Loop::Continue(redirect_uri))
                    },
                    Ok(GetResult::Ok(res)) => Ok(future::Loop::Break(res.body())),
                    Err(err) => Err(err),
                }
            })
    }).and_then(|body| {
        body.fold(Vec::new(), |mut acc, chunk| {
            acc.extend_from_slice(&chunk);
            future::ok::<_, hyper::Error>(acc)
        }).map_err(Error::from)
    }))
}

