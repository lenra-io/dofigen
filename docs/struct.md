# Dofigen struct reference

This is the reference for the Dofigen configuration file structure.

The Dofigen struct is a YAML or JSON object that can be used to generate a Dockerfile.

The struct is permissive in order to make it easy to write and read.
For example, some objects can be parsed from string and all arrays can be parsed from single element.

- [Dofigen struct reference](#dofigen-struct-reference)
	- [Dofigen](#dofigen)
	- [Extend](#extend)
	- [Stage](#stage)
	- [FromContext](#fromcontext)
	- [User](#user)
	- [CopyResource](#copyresource)
	- [Run](#run)
	- [Cache](#cache)
	- [Bind](#bind)
	- [TmpFs](#tmpfs)
	- [Secret](#secret)
	- [Ssh](#ssh)
	- [Healthcheck](#healthcheck)
	- [ImageName](#imagename)
	- [Copy](#copy)
	- [CopyContent](#copycontent)
	- [AddGitRepo](#addgitrepo)
	- [Add](#add)
	- [CopyOptions](#copyoptions)
	- [Port](#port)

## Dofigen

This is the root object of the Dofigen configuration file.

It extends the [Extend](#extend) and [Stage](#stage) structures.

| Field | Type | Description |
| --- | --- | --- |
| `context` | string[] | The context of the Docker build. This is used to generate a `.dockerignore` file. |
| `ignore` | string[] | The elements to ignore from the build context. This is used to generate a `.dockerignore` file. |
| `globalArg` | map<string, string> | The build args that can be used in the global scope. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#arg) and [variable scoping](https://docs.docker.com/build/building/variables/#scoping). |
| `builders` | map<string, [Stage](#stage)> | The builder stages of the Dockerfile. |
| `entrypoint` | string[] | The entrypoint of the Dockerfile. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#entrypoint). |
| `cmd` | string[] | The default command of the Dockerfile. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#cmd). |
| `volume` | string[] | Create volume mounts. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#volume). |
| `expose` | [Port](#port)[] | The ports exposed by the Dockerfile. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#expose). |
| `healthcheck` | [Healthcheck](#healthcheck) | The healthcheck of the Dockerfile. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#healthcheck). |

## Extend

This let you extend a struct from local or remote files.

| Field | Type | Description |
| --- | --- | --- |
| `extend` | string or string[] | The files to extend. |

## Stage

This represents a Dockerfile stage.

It extends the [Run](#run) structure.

| Field | Type | Description |
| --- | --- | --- |
| `from...` | [FromContext](#fromcontext) | The base of the stage. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#from). |
| `label` | map<string, string> | Add metadata to an image. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#label) |
| `user` | [User](#user) | The user and group of the stage. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#user). |
| `workdir` | string | The working directory of the stage. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#workdir). |
| `arg` | map<string, string> | The build args that can be used in the stage. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#arg). |
| `env` | map<string, string> | The environment variables of the stage. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#env). |
| `copy` | [CopyResource](#copyresource) or [CopyResource](#copyresource)[] | The copy instructions of the stage. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#copy) and [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#add). |
| `root` | [Run](#run) | The run instructions of the stage as root user. |

## FromContext

This represents a context origin.

Possible fields are:

- `fromImage` ([ImageName](#imagename)) : A Docker image.
- `fromBuilder` (string) : A builder from the same Dofigen file.
- `fromContext`: (string) : A Docker build context. See https://docs.docker.com/reference/cli/docker/buildx/build/#build-context

## User

This represents user and group definition.

It can be parsed from string.

| Field | Type | Description |
| --- | --- | --- |
| `user` | string | The user name or ID. |
| `group` | string | The group name or ID. |

## CopyResource

This represents the COPY/ADD instructions in a Dockerfile.

It can be one of the following objects:

- [Copy](#copy) : A copy instruction.
- [CopyContent](#copycontent) : A copy instruction from file content.
- [Add](#add) : An add instruction.
- [AddGitRepo](#addgitrepo) : An add instruction from a git repository.

## Run

This represents a run command.

| Field | Type | Description |
| --- | --- | --- |
| `run` | string or string[] | The commands to run. |
| `shell` | string or string[] | The shell to use for the RUN command. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#shell). |
| `cache` | [Cache](#cache)[] | The cache definitions during the run. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#run---mounttypecache). |
| `bind` | [Bind](#bind)[] | The file system bindings during the run. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#run---mounttypebind). |
| `tmpfs` | [TmpFs](#tmpfs)[] | This mount type allows [mounting tmpfs](https://docs.docker.com/reference/dockerfile/#run---mounttypetmpfs) in the build container. |
| `secret` | [Secret](#secret)[] | This allows the build container to access secret values, such as tokens or private keys, without baking them into the image. |
| `ssh` | [Ssh](#ssh)[] | This allows the build container to access SSH keys via SSH agents, with support for passphrases. |
| `network` | `default`, `none` or `host` | This allows control over which networking environment the command is run in. |
| `security` | `sandbox` or `insecure` | The default security mode is sandbox. With `security: insecure`, the builder runs the command without sandbox in insecure mode, which allows to run flows requiring elevated privileges (e.g. containerd). |

## Cache

This represents a cache definition during a run.

It can be parsed from string.

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | The id of the cache. This is used to share the cache between different stages. |
| `target` | string | The target path of the cache. |
| `readonly` | boolean | Defines if the cache is readonly. |
| `sharing` | "shared" or "private" or "locked" | The sharing strategy of the cache. |
| `from...` | [FromContext](#fromcontext) | The base of the cache mount. |
| `source` | string | Subpath in the from to mount. |
| `chmod` | string or integer | The permissions of the cache. |
| `chown` | [User](#user) | The user and group that own the cache. |

## Bind

This represents file system binding during a run.

It can be parsed from string.

| Field | Type | Description |
| --- | --- | --- |
| `target` | string | The target path of the bind. |
| `from...` | [FromContext](#fromcontext) | The base of the cache mount. |
| `source` | string | Subpath in the from to mount. |
| `readwrite` | boolean | Defines if the bind is read and write. |

## TmpFs

This mount type allows [mounting tmpfs](https://docs.docker.com/reference/dockerfile/#run---mounttypetmpfs) in the build container.

It can be parsed from string.

| Field | Type | Description |
| --- | --- | --- |
| `target` | string | Mount path. |
| `size` | string | Specify an upper limit on the size of the filesystem. |

## Secret

This mount type allows the build container to access secret values, such as tokens rivate keys, without baking them into the image.
By default, the secret is mounted as a file. You can also mount the secret as an ronment variable by setting the env option.
See https://docs.docker.com/reference/dockerfile/#run---mounttypesecret

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | ID of the secret. Defaults to basename of the target path. |
| `target` | string | Mount the secret to the specified path. Defaults to /run/secrets/ + id if unset and if env is also unset. |
| `env` | string | Mount the secret to an environment variable instead of a file, or both. |
| `required` | string | If set to true, the instruction errors out when the secret is unavailable. Defaults to false. |
| `mode` | string | File mode for secret file in octal. Default `0400`. |
| `uid` | int | User ID for secret file. Default 0. |
| `gid` | int | Group ID for secret file. Default 0. |

## Ssh

This mount type allows the build container to access SSH keys via SSH agents, with support for passphrases.
See https://docs.docker.com/reference/dockerfile/#run---mounttypessh

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | ID of SSH agent socket or key. Defaults to "default". |
| `target` | string | SSH agent socket path. Defaults to /run/buildkit/ssh_agent.${N}. |
| `required` | If set to true, the instruction errors out when the key is unavailable. Defaults to false. |
| `mode` | string | File mode for socket in octal. Default `0600`. |
| `uid` | int | User ID for socket. Default 0. |
| `gid` | int | Group ID for socket. Default 0. |


## Healthcheck

This represents the Dockerfile healthcheck instruction.

| Field | Type | Description |
| --- | --- | --- |
| `cmd` | string | The test command to run. |
| `interval` | string | The time between running the check (ms|s|m|h). |
| `timeout` | string | The time to wait before considering the check to have hung (ms|s|m|h). |
| `startPeriod` | string | The time to wait for the container to start before starting health-retries countdown (ms|s|m|h). |
| `retries` | int | The number of consecutive failures needed to consider a container as unhealthy. |

## ImageName

This represents a Docker image name.

It can be parsed from string.

| Field | Type | Description |
| --- | --- | --- |
| `host` | string | The host of the image registry. |
| `port` | int | The port of the image registry. |
| `path` | string | The path of the image repository. |
| `platform` | string | The optional platform option can be used to specify the platform of the image in case FROM references a multi-platform image. This option is ignored if not for a stage (build or runtime) `fromImage`. |

The version of the image can also be set with one of the following fields:

- `tag` : The tag of the image.
- `digest` : The digest of the image.

## Copy

This represents the COPY instruction in a Dockerfile.

It extends the [CopyOptions](#copyoptions) structure.

Can be parsed from string.

| Field | Type | Description |
| --- | --- | --- |
| `from...` | [FromContext](#fromcontext) | The origin of the copy. See https://docs.docker.com/reference/dockerfile/#copy---from |
| `paths` | string[] | The paths to copy. |

## CopyContent

This represents the COPY instruction in a Dockerfile based on file content.

It extends the [CopyOptions](#copyoptions) structure, but the target field is required.

Can be parsed from string.

| Field | Type | Description |
| --- | --- | --- |
| `content` | string | Content of the file to copy. |
| `substitute` | boolean | If true, replace variables in the content at build time. Default is true. |

## AddGitRepo

This represents the ADD instruction in a Dockerfile specific for Git repositories.

It extends the [CopyOptions](#copyoptions) structure.

Can be parsed from string.

| Field | Type | Description |
| --- | --- | --- |
| `repo` | string | The URL of the Git repository. |
| `keepGitDir` | boolean | Keep the git directory. See https://docs.docker.com/reference/dockerfile/#add---keep-git-dir |
| `checksum` | string | The checksum of the files. |

## Add

This represents the ADD instruction in a Dockerfile for files from URLs or to uncompress an archive.

It extends the [CopyOptions](#copyoptions) structure.

Can be parsed from string.

| Field | Type | Description |
| --- | --- | --- |
| `files` | string[] | The source files to add. |
| `checksum` | string | The checksum of the files. See https://docs.docker.com/reference/dockerfile/#add---checksum |
| `unpack` | boolean | The unpack flag controls whether or not to automatically unpack tar archives (including compressed formats like gzip or bzip2) when adding them to the image. |

## CopyOptions

This represents the options of a COPY/ADD instructions.

| Field | Type | Description |
| --- | --- | --- |
| `target` | string | The target path of the copied files. |
| `chown` | [User](#user) | The user and group that own the copied files. See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod |
| `chmod` | string or integer | The permissions of the copied files. See https://docs.docker.com/reference/dockerfile/#copy---chown---chmod |
| `link` | boolean | Use of the link flag. See https://docs.docker.com/reference/dockerfile/#copy---link |

## Port

This represents a port definition.

It can be parsed from string.

| Field | Type | Description |
| --- | --- | --- |
| `port` | int | The port number. |
| `protocol` | "tcp" or "udp" | The protocol of the port. |
