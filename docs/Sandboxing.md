# Sandboxing
Because the webserver accesses files on the filesystem, it is important to
restrict the files that it can access.

This document does not cover general sandboxing tools (virtualisation), like
WSL2, Docker,

## Problem
The webserver needs to access files on the filesystem, but it should not be
able to access files that it should not be able to access.

It needs to do the following:
1. At initialisation
    1.  Read files from the filesystem, only in `wwwroot`.
    2.  Read the certificate and private key files
    3.  Open TCP and UDP sockets
2. At runtime
   1. Read files from the filesystem, only in `wwwroot`.
   2. Read the file statistics:
      1. Modification time for `Last-Modified`, `ETag` and caching
      2. File size for buffering
   3. `accept`, `read` and `write` from client sockets,
   4. Get the current system time, for the `Date` header.

The following is a non-exhaustive syscalls it shouldn't make:
1. Execute files and executables or run commands
2. Write or create files
3. Change file permissions and statistics
4. Open sockets to outside (but this is more a job of the firewall)
5. Read files outside of `wwwroot`, especially:
   1. The TLS files
   2. `/etc/passwd` and `/etc/shadow`
   3. `/var/log/`, `/dev/`

### Exceptions
Unfortunately, applying all of the above is not so simple, for example for the
following use cases:
1. The metrics/analytics system, it should be able to write to a file or database
2. Logging
3. Automatic certificate renewal or generation (e.g. ACME), but this can be a
   separate process that notifies the webserver (or just restarts it).


## Solutions on OpenBSD
OpenBSD has a feature called [pledge](https://man.openbsd.org/pledge.2) that
allows the programmer to restrict the system calls that a program can make.
This way, we can disable all unnecessary calls, and only allow the ones that
are needed.

Also, we can use the [unveil](https://man.openbsd.org/unveil.2) system call to
restrict the files that a program can access.


## Solutions on Windows
Currently, there are no native Windows features allowing the programmer to
restrict system calls. The solution to this problem might be to run it using
[Windows Subsystem for Linux 2](https://learn.microsoft.com/en-us/windows/wsl/about),
which allows `Linux` kernels to be run on `Windows`, providing all the
protection that it supports.


## Possible Solutions on Linux
On Linux, there are a plethora of solutions, with each having their own
advantages and disadvantages.

### [AppArmor](https://www.apparmor.net/)
AppArmor allows the administrator, not the developer, to set certain
restrictions related to e.g. file access. Since it is an admistrative task,
at the moment of writing it will not be explored further.

### [Capabilities](https://www.man7.org/linux/man-pages/man7/capabilities.7.html)
On UNIX systems, opening sockets on ports lower than 1024 requires root
privileges. However, it is possible to give a program the ability to open
sockets on these ports, without giving it root privileges.

This is done by using the `CAP_NET_BIND_SERVICE` capability. This capability
can be given to a program using the `setcap` command.

### [seccomp](https://man7.org/linux/man-pages/man2/seccomp.2.html)
seccomp allows the program to enter a sort of 'secure' mode, which disables all
system calls, except a few allowed ones.

#### SECCOMP_SET_MODE_STRICT
This mode allows only the `read` and `write` syscalls on _already open_ file
descriptors.

This doesn't sound that interesting, since we would still need to update the
cache for when files change, but we could circumvent this issue by having a
separate (child) process which _can_ read from disk, and sending updates to the
server using UNIX sockets or pipes.

However, also the `accept` syscall is disabled, rendering this mode mostly
useless, except for maybe a super-secure mode, making a separate process for
each connection, having that process use the `SECCOMP_SET_MODE_STRICT` mode.

### [SELinux](https://selinuxproject.org/page/FAQ)
SELinux allows the administrator, not the developer, to set certain
restrictions related to e.g. file access. Since it is an admistrative task,
at the moment of writing it will not be explored further.


### [Landlock](https://www.kernel.org/doc/html/latest/userspace-api/landlock.html)
Landlock is a mechanism for programs to secure themselves by restricting file
access, with in the future more controls like network restrictions.


## Comparison of mechanisms

