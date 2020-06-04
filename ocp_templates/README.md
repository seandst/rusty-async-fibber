Yamls for building up the mock "microservice" in OpenShift. The order is:

- imageStream.yml
- buildConfig.yml
- Start a build using the build config so that the deployment config
  references an existing image
- deploymentConfig.yml
- service.yml

Openshift (at least the one I'm using) doesn't let you set up ingress
for a non-HTTP service (which this is), so access to the fibber is
only given to cluster members already inside the cluster. So, from
any pod that's got the ability to talk to the service addr:port, you
can telnet/netcat/socat queries to the fibber service:

```
# socat was already installed on my jenkins pod, so that's what I went with
sh-4.2$ socat - TCP:172.30.52.253:1234 <<< 3
Ok: 2
sh-4.2$ socat - TCP:172.30.52.253:1234 <<< test
Err: 'test' is not an integer followed by a newline
sh-4.2$ dd if=/dev/urandom bs=1024 count=1 status=none | socat - TCP:172.30.52.253:1234
Err: Bro I can't even read that
```
