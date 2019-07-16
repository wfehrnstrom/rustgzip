# Documentation

This is intended as both a guide to the architecture of gzip in rust and a
indication of what has been done in the port so far.

## Options Partially or Wholly implemented

gzip

  --stdout/--to-stdout, -c
  --decompress/--uncompress, -d
  --force, -f
  --help, -h
  --keep, -k
  --license, -L
  --list, -l
  --quiet, -q
  --recursive, -r
  --suffix [SUF], -S [SUF]
  --verbose, -v
  --version, -V
  --fast
  --best
  but instead of -[n], this gzip has a flag --level [LVL]
  This will be removed in the future, and -[n] added.

## Options remaining to be implemented

Currently compressing to and decompressing from multipart gzip files is not
implemented. Also not implemented is the ability to pass multiple gzip streams
through stdin, have them all compressed together and outputted as a multipart
gzip file.

gzip

  --no-name, -n
  --name, -N
  --rsyncable
  --synchronous
