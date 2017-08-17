# Changelog

(see http://keepachangelog.com http://semver.org)
(critical comment about semver: https://gist.github.com/jashkenas/cbd2b088e20279ae2c8e)

## Unreleased

## [0.2.0] - 2017-08-17
* Major change in the interface of the service. Papers now uploads the rendered
  PDF and the build artifact directly to an S3 bucket, which requires
  additional configuration on your part. The `callback_url` will now receive a
  `Summary`, which contains either an `error` or a `file` field with the
  presigned url to the rendered PDF, and an `s3_folder` field for the location
  of the debugging output.
* Rename all templates and merge from [name].tex to [name].tex.tera
* Better error reporting

## [0.1.0] - 2017-05-23
* Initial release
