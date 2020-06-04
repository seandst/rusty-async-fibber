# wat

My first little foray into rust, which is generally useless
and presumably reflects my naivete when it comes to developing in
rust. The async fibber is an example threaded socket server that
does fibonacci stuff via TCP on port 1234 (yeah I hardcoded the
port).

# dawkah?

Since I've been working with OpenShift so much recently, I baked it up as
an openshift "microservice", statically compiled into a container where
it is the only occupant, and is built as a single layer. The container
image weighs in at just over 3MB, but of course would be larger depending
on the size of the app baked in. The fibber itself is about 12kB of code,
so presumably the rest is all of the statically linked stuff that gets
shoved into the bin by rustc.

For fun, here's what `tar -tvf` has to say about the container layer
when exported as a tar file:

```
-rwxr-xr-x root/root         0 2020-06-04 10:10 .dockerenv
-rwxr-xr-x root/root   3230848 2020-06-03 14:15 asyncfibber
drwxr-xr-x root/root         0 2020-06-04 10:10 dev/
-rwxr-xr-x root/root         0 2020-06-04 10:10 dev/console
drwxr-xr-x root/root         0 2020-06-04 10:10 dev/pts/
drwxr-xr-x root/root         0 2020-06-04 10:10 dev/shm/
drwxr-xr-x root/root         0 2020-06-04 10:10 etc/
-rwxr-xr-x root/root         0 2020-06-04 10:10 etc/hostname
-rwxr-xr-x root/root         0 2020-06-04 10:10 etc/hosts
lrwxrwxrwx root/root         0 2020-06-04 10:10 etc/mtab -> /proc/mounts
-rwxr-xr-x root/root         0 2020-06-04 10:10 etc/resolv.conf
drwxr-xr-x root/root         0 2020-06-04 10:10 proc/
drwxr-xr-x root/root         0 2020-06-04 10:10 sys/
```

So as far as a "microservice" goes, it's pretty micro.

# OpenShift?

Templates for building and running this thing in openshift can be found in
the [ocp_templates](ocp_templates/) dir.

# but how?

Assuming docker, though this should work just as well with podman,
`docker build -t asyncfibber .`, and then `docker run -P asyncfibber`.
Then you can talk to it over a tcp socket (so telnet, netcat/nc, socat,
write a program, whatever) to get some fibonacci numbers.

The interface is:
  - Send an int, followed by a newline
  - Send whatever you want, but don't expect a good answer

You will receive back either:
  - `Ok: <the answer>`
  - `Err: <an error message explaining what happened or berating you>`

I haven't spent any time with docker trying to figure out how to easily
stop it from the shell. `pkill -9 asyncfibber` works great, though, if
you don't mind being rude to all the asyncfibbers that you might be
running.
