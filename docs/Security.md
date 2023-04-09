# Security
This document describes common/possible security flaws, and how this project
addresses them. These flaws both include weaknesses in the server software or
network protocols, and misconfiguration by the server adminstrator.

Administrators should be aware of the dangers of serving arbitrary files, but it
is still advantageous to have a web server with appropriate security measures in
case. As they are the authority of traffic between the outside world (web) and
inside world (operating system, intra/extranet), they have the opportunity to
provide mechanisms of a last resort.

This document can be a useful resource, even for other projects, to be aware of
the possible dangers of running a web service. Because of this, the document
should first describe the possible dangers of a certain aspect of a web server,
and then describe the ways **Servente** addresses them. Furthermore, it should
also cover attacks that aren't necessarily applicable for Servente.

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

## Scripts as Textual Content
Employing CGI, or [Common Gateway Interface](https://www.rfc-editor.org/rfc/rfc3875)
is a common practice in the World Wide Web, but some servers might not correctly
identify those scripts as such. These servers might render or otherwise serve
the commonly textual content of those scripts to the user. Doing so often
exposes inner workings of the website or related system, in the worst case
leaking confidential or otherwise sensitive information to an attacker.

### Current Mitigations
Servente addresses this issue by having the following mitigations in place:
1. Executable files (POSIX, i.e. macOS, Linux, *BSD, *NIX) are never served to
   the end user and won't be present in cache. This prevents the contents of
   those scripts from being leaked.

### Future Prevention Mechanisms
The following mechanisms might be advantageous to implement in the future:
1. Files that are uncommonly used in the front-end web, and would normally
   fulfill the responsibility of handling requests or otherwise provide
   system infrastructure can be rejected based on file type.

   **Example**
   `.php` files are normally processed by a [PHP](https://www.php.net/)
   interpreter, and are rarely verbatimly served as textual content. These files
   often incorporate sensitive information about the workings of a system,
   sometimes even containing login credentials for accessing a database.

   **Exception**
   There are legitimate reasons for exposing scripts as textual resources to the
   client, for example a version control system like a web-based
   [Git](https://git-scm.com/) interface, which host the source code of
   programs. The solution for this problem is having a configurable toggle for
   this mitigation, allowing the administrator to opt-out of this behavior.


### Infeasible Mitigations
**JavaScript** and **TypeScript** files are nowadays widely used as the back-end
for a lot of websites. Since they are also used to provide client-side
functionality to otherwise static sites, it is impossible to filter these files
based on heuristics like the file extension (`*.js`, `*.ts`) and MIME-sniffing.

## Slowloris attacks
**To be mitigated.** Currently, having asynchronous I/O circumvents this issue
somewhat, but there should be a maximum time since the first byte to the last
byte.
