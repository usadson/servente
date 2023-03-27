# Security
This document describes common/possible security flaws, and how this project
addresses them.

## Attacks based on file path
Since the resource-target might be a relative path, inserting an `/../`
component might lead to path traversal on naive webservers. This is fully
circumented, checking the components for `ParentDir`, and checking whether or
not the path actually starts with the absolute path of the `wwwroot` directory.

Example attacks:
* `/../../../../../../../etc/shadow` - leaking the passwords on UNIX-like
  systems
* `/../../../../var/www/html/index.html` - leaking the directory structure of
  the underlying system.

## Attacks based on temporary and accidental files
Some files are currently being filtered on file extension, like `.log` and
`.tmp`. In the future, it might be useful to filter out files based on magics
when the extension doesn't reflect the contents, and filter out CGI files that
are not supported, but mistakenly inserted there. This could lead to leaking
sensitive security information, in the worst case leaking credentials.

## Slowloris attacks
**To be mitigated.** Currently, having asynchronous I/O circumvents this issue
somewhat, but there should be a maximum time since the first byte to the last
byte.
