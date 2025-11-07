### Extension Process

1. **Reference**: The `extend` field specifies the files to be extended.
2. **Merge**: The referenced files are merged into the current file. If there are conflicts, the values in the current file take precedence.
3. **Apply**: The merged configuration is applied to the struct, allowing you to build upon the base configurations defined in the extended files.



### How the `extend` Field References Other YAML Files

The `extend` field can reference both local and remote YAML files. Local files are specified using relative or absolute paths, while remote files are specified using URLs. The `extend` field can accept a single file path or an array of file paths, allowing you to extend from multiple files.


### Syntax Examples


#### Local File Extension

```yaml
struct:
  extend: "path/to/local/file.yaml"
```


#### Remote File Extension

```yaml
struct:
  extend: "https://example.com/path/to/remote/file.yaml"
```


#### Multiple File Extension

```yaml
struct:
  extend:
    - "path/to/local/file.yaml"
    - "https://example.com/path/to/remote/file.yaml"
```


### Practical Examples


#### Example 1: Extending a Base Configuration

```yaml
# base.yaml
struct:
  fromImage: "nginx:latest"
  user: "nginx"
  workdir: "/usr/share/nginx/html"

# main.yaml
struct:
  extend: "base.yaml"
  env:
    NGINX_VERSION: "1.21.6"
```


#### Example 2: Overriding Specific Fields

```yaml
# base.yaml
struct:
  fromImage: "nginx:latest"
  user: "nginx"
  workdir: "/usr/share/nginx/html"

# main.yaml
struct:
  extend: "base.yaml"
  fromImage: "nginx:alpine"
  user: "root"
```


#### Example 3: Handling Complex Structures

```yaml
# base.yaml
struct:
  env:
    - NGINX_VERSION: "1.21.6"
    - APP_ENV: "production"
  copy:
    - source: "./config"
      target: "/etc/nginx/conf.d"

# main.yaml
struct:
  extend: "base.yaml"
  env:
    - NGINX_VERSION: "1.22.0"
    - APP_ENV: "staging"
  copy:
    - source: "./custom-config"
      target: "/etc/nginx/conf.d"
```



The `extend` field allows you to reference and extend other YAML files, enabling you to reuse and modify configurations without duplicating them. This is particularly useful for maintaining consistency across multiple Dockerfile stages or for sharing common configurations between different projects.

### How the `extend` Field Affects Structure and Data

When you use the `extend` field, the referenced YAML files are merged into the current file. If there are conflicts, the values in the current file take precedence. This merging process affects the structure and data of the extended file in the following ways:

- **Field Merging**: Simple fields are merged, with the current file's values overriding those from the extended files.
- **Array Handling**: Arrays are concatenated. If the current file has an array field, the values from the extended files are appended to it.
- **Map Handling**: Maps are merged, with the current file's key-value pairs overriding those from the extended files. If a key exists in both the current file and the extended file, the value from the current file is used.

This merging process ensures that the extended file retains the structure and data from the referenced files while allowing for customization and extension.


The `extend` field allows you to reference and extend other YAML files, enabling you to reuse and modify configurations without duplicating them. This is particularly useful for maintaining consistency across multiple Dockerfile stages or for sharing common configurations between different projects.


| Field | Type | Description |
| --- | --- | --- |
| `extend` | string or string[] | The files to extend. |


### Usage Examples


Basic extension:

```yaml
struct:
  extend: "base.yaml"
```

Extending multiple files:

```yaml
struct:
  extend:
    - "base.yaml"
    - "additional.yaml"
```

### Extension Process

1. **Reference**: The `extend` field specifies the files to be extended.
2. **Merge**: The referenced files are merged into the current file. If there are conflicts, the values in the current file take precedence.
3. **Apply**: The merged configuration is applied to the struct, allowing you to build upon the base configurations defined in the extended files.


## Extend

The `extend` field allows you to reference and extend other YAML files, enabling you to reuse and modify configurations without duplicating them. This is particularly useful for maintaining consistency across multiple Dockerfile stages or for sharing common configurations between different projects.

### Usage Examples

Basic extension:

```yaml
struct:
  extend: "base.yaml"
```

Extending multiple files:

```yaml
struct:
  extend:
    - "base.yaml"
    - "additional.yaml"
```

### Extension Process

1. **Reference**: The `extend` field specifies the files to be extended.
2. **Merge**: The referenced files are merged into the current file. If there are conflicts, the values in the current file take precedence.
3. **Apply**: The merged configuration is applied to the struct, allowing you to build upon the base configurations defined in the extended files.


### How the `extend` Field References Other YAML Files

The `extend` field can reference both local and remote YAML files. Local files are specified using relative or absolute paths, while remote files are specified using URLs. The `extend` field can accept a single file path or an array of file paths, allowing you to extend from multiple files.


### Syntax Examples


#### Local File Extension

```yaml
struct:
  extend: "path/to/local/file.yaml"
```


#### Remote File Extension

```yaml
struct:
  extend: "https://example.com/path/to/remote/file.yaml"
```


#### Multiple File Extension

```yaml
struct:
  extend:
    - "path/to/local/file.yaml"
    - "https://example.com/path/to/remote/file.yaml"
```


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


## FromImage


The `fromImage` field specifies the base image for a resource in Dofigen. It is used to define the starting point for Dockerfile generation, allowing you to build upon an existing image. This field is crucial for creating Dockerfiles that extend from specific base images, ensuring consistency and reusability across your Docker builds.


### Usage


To use the `fromImage` field, you need to specify the image name and optionally a tag or digest. Here is an example of how to use it in a Dofigen file:


```yaml
fromImage: nginx:latest
```


### Importance in Dockerfile Generation


The `fromImage` field is essential in the Dockerfile generation process as it determines the base layer of your Docker image. It allows you to leverage existing images, reducing the amount of work needed to create a new image from scratch. By specifying a base image, you can ensure that your Dockerfile starts with a known and stable environment, making it easier to manage dependencies and configurations.


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
| `cache` | [Cache](#cache)[] | The cache definitions during the run. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#run---mounttypecache). |
| `bind` | [Bind](#bind)[] | The file system bindings during the run. See [Dockerfile reference](https://docs.docker.com/reference/dockerfile/#run---mounttypebind). |


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


## Add


This represents the ADD instruction in a Dockerfile for files from URLs or to uncompress an archive.


It extends the [CopyOptions](#copyoptions) structure.


Can be parsed from string.


| Field | Type | Description |
| --- | --- | --- |
| `files` | string[] | The source files to add. |
| `checksum` | string | The checksum of the files. See https://docs.docker.com/reference/dockerfile/#add---checksum |


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


## Extend


The `extend` field allows you to reference and extend other YAML files, enabling you to reuse and modify configurations without duplicating them. This is particularly useful for maintaining consistency across multiple Dockerfile stages or for sharing common configurations between different projects.


### Usage Examples


Basic extension:

```yaml
struct:
  extend: "base.yaml"
```

Extending multiple files:

```yaml
struct:
  extend:
    - "base.yaml"
    - "additional.yaml"
```


### Extension Process


1. **Reference**: The `extend` field specifies the files to be extended.
2. **Merge**: The referenced files are merged into the current file. If there are conflicts, the values in the current file take precedence.
3. **Apply**: The merged configuration is applied to the struct, allowing you to build upon the base configurations defined in the extended files.


### How the `extend` Field References Other YAML Files


The `extend` field can reference both local and remote YAML files. Local files are specified using relative or absolute paths, while remote files are specified using URLs. The `extend` field can accept a single file path or an array of file paths, allowing you to extend from multiple files.


### Syntax Examples


#### Local File Extension

```yaml
struct:
  extend: "path/to/local/file.yaml"
```


#### Remote File Extension

```yaml
struct:
  extend: "https://example.com/path/to/remote/file.yaml"
```


#### Multiple File Extension

```yaml
struct:
  extend:
    - "path/to/local/file.yaml"
    - "https://example.com/path/to/remote/file.yaml"
```


### Practical Examples


#### Example 1: Extending a Base Configuration

```yaml
# base.yaml
struct:
  fromImage: "nginx:latest"
  user: "nginx"
  workdir: "/usr/share/nginx/html"

# main.yaml
struct:
  extend: "base.yaml"
  env:
    NGINX_VERSION: "1.21.6"
```


#### Example 2: Overriding Specific Fields

```yaml
# base.yaml
struct:
  fromImage: "nginx:latest"
  user: "nginx"
  workdir: "/usr/share/nginx/html"

# main.yaml
struct:
  extend: "base.yaml"
  fromImage: "nginx:alpine"
  user: "root"
```


#### Example 3: Handling Complex Structures

```yaml
# base.yaml
struct:
  env:
    - NGINX_VERSION: "1.21.6"
    - APP_ENV: "production"
  copy:
    - source: "./config"
      target: "/etc/nginx/conf.d"

# main.yaml
struct:
  extend: "base.yaml"
  env:
    - NGINX_VERSION: "1.22.0"
    - APP_ENV: "staging"
  copy:
    - source: "./custom-config"
      target: "/etc/nginx/conf.d"
```
Patches in Dofigen support several types of operations:

- **Insert**: Adds new data to the structure.
- **Delete**: Removes existing data from the structure.
- **Update**: Modifies the values of existing data.

These operations allow for precise control over the data structure, enabling targeted modifications that maintain the integrity of the configuration while allowing for flexibility and scalability.

### Map Patching

Map patching in Dofigen allows for precise modifications to key-value pairs within YAML structures. This process involves adding, removing, or updating key-value pairs in a map. The patching system ensures that changes are applied in a controlled manner, maintaining the integrity of the data structure.

#### Adding Key-Value Pairs

To add a new key-value pair to a map, you can use the following YAML syntax:

```yaml
map:
  +key: value
```

This will add the key `key` with the value `value` to the map.

#### Removing Key-Value Pairs

To remove a key-value pair from a map, you can use the following YAML syntax:

```yaml
map:
  -key: null
```

This will remove the key `key` from the map.

#### Updating Key-Value Pairs

To update the value of an existing key in a map, you can use the following YAML syntax:

```yaml
map:
  key: new_value
```

This will update the value of the key `key` to `new_value`.

### Example

Here is an example of a YAML structure before and after applying map patches:

**Before:**

```yaml
config:
  version: 1.0
  settings:
    theme: dark
    language: en
```

**After Applying Patches:**

```yaml
config:
  version: 2.0
  settings:
    theme: light
    +new_setting: enabled
    -language: null
```

In this example, the version is updated, the theme is changed, a new setting is added, and the language setting is removed.

Here's a practical example of creating and applying a patch to a data structure:

#### Before Patch
```yaml
# Base YAML structure
config:
  version: 1.0
  settings:
    theme: dark
    language: en
```

#### Patch
```yaml
# Patch to apply
config:
  settings:
    theme: light
    new_setting: enabled
```

#### After Patch
```yaml
# Resulting YAML structure after applying the patch
config:
  version: 1.0
  settings:
    theme: light
    language: en
    new_setting: enabled
```

## Dofigen

This is the reference for the Dofigen configuration file structure.


The Dofigen struct is a YAML or JSON object that can be used to generate a Dockerfile.


The struct is permissive in order to make it easy to write and read.
For example, some objects can be parsed from string and all arrays can be parsed from single element.


- [Dofigen struct reference](#dofigen-struct-reference)
	- [YAML Patch System](#yaml-patch-system)
	- [Dofigen](#dofigen)
	- [Extend](#extend)
	- [Stage](#stage)
	- [FromContext](#fromcontext)
	- [User](#user)
	- [CopyResource](#copyresource)
	- [Run](#run)
	- [Cache](#cache)
	- [Bind](#bind)
	- [Healthcheck](#healthcheck)
	- [ImageName](#imagename)
	- [Copy](#copy)
	- [CopyContent](#copycontent)
	- [AddGitRepo](#addgitrepo)
	- [Add](#add)
	- [CopyOptions](#copyoptions)
	- [Port](#port)


## YAML Patch System


The YAML patch system in Dofigen allows for extending and modifying YAML structures by applying patches. This system is useful for reusing and modifying configurations without duplicating them.

## Patches

A patch in Dofigen represents a set of changes to be applied to a data structure. Patches can include operations such as adding new fields, removing existing fields, or modifying the values of existing fields. This allows for flexible and controlled modifications to YAML configurations without duplicating entire configurations. Patches are applied sequentially, with each patch operation modifying the data structure incrementally. This approach ensures that changes are made in a predictable and manageable way, making it easier to maintain and update configurations over time.

### Patch Operations

Patches in Dofigen support several types of operations:

- **Insert**: Adds new data to the structure.
- **Delete**: Removes existing data from the structure.
- **Update**: Modifies the values of existing data.

These operations allow for precise control over the data structure, enabling targeted modifications that maintain the integrity of the configuration while allowing for flexibility and scalability.

### Merge Strategy

Dofigen uses a strategic approach to resolve conflicts when multiple patches are applied to the same data structure. The merge strategy prioritizes the most recent patch, ensuring that the final configuration reflects the latest changes. This approach maintains data integrity and allows for flexible updates without overwriting critical information. By applying patches in a controlled manner, Dofigen ensures that the configuration remains consistent and up-to-date.

### Usage Example

Here's an example of how patches are created and applied to a data structure:

```yaml
# Base data structure
base:
  name: "Base Configuration"
  version: "1.0"
  settings:
    theme: "dark"
    notifications: true

# Patch to apply
patch:
  name: "Updated Configuration"
  settings:
    theme: "light"
    notifications: false
    new_feature: true

# Result after applying the patch
result:
  name: "Updated Configuration"
  version: "1.0"
  settings:
    theme: "light"
    notifications: false
    new_feature: true
```

## Extend Field

The `extend` field allows you to reference and extend other YAML files, enabling you to reuse and modify configurations without duplicating them. This is particularly useful for maintaining consistency across multiple Dockerfile stages or for sharing common configurations between different projects.

### How the `extend` Field Affects Structure and Data

When you use the `extend` field, the referenced YAML files are merged into the current file. If there are conflicts, the values in the current file take precedence. This merging process affects the structure and data of the extended file in the following ways:

- **Field Merging**: Simple fields are merged, with the current file's values overriding those from the extended files.
- **Array Handling**: Arrays are concatenated. If the current file has an array field, the values from the extended files are appended to it.
- **Map Handling**: Maps are merged, with the current file's key-value pairs overriding those from the extended files. If a key exists in both the current file and the extended file, the value from the current file is used.

This merging process ensures that the extended file retains the structure and data from the referenced files while allowing for customization and extension.

### Usage Examples

Basic extension:

```yaml
struct:
  extend: "base.yaml"
```

Extending multiple files:

```yaml
struct:
  extend:
    - "base.yaml"
    - "additional.yaml"
```

### Extension Process

1. **Reference**: The `extend` field specifies the files to be extended.
2. **Merge**: The referenced files are merged into the current file. If there are conflicts, the values in the current file take precedence.
3. **Apply**: The merged configuration is applied to the struct, allowing you to build upon the base configurations defined in the extended files.



In this example, the patch updates the `name` field, modifies the `theme` and `notifications` settings, and adds a new `new_feature` field. The `version` field remains unchanged as it was not included in the patch.

## Array Patching

This represents the patching of arrays in Dofigen YAML files.

### Array Patch Examples

Here are some examples of how to patch arrays in Dofigen YAML files:

```yaml
# Add an element to an array
array:
  +: "new_element"

# Remove an element from an array
array:
  2: null

# Replace an element in an array
array:
  1: "updated_element"
```

### Special Considerations

- **Empty Arrays**: When patching an empty array, you can add elements directly without specifying positions.
- **Duplicate Elements**: Dofigen allows duplicate elements in arrays, but you should ensure that the configuration logic can handle duplicates appropriately.



## Map Patching

Map patching in Dofigen allows for precise modifications to key-value pairs within YAML structures. This process involves adding, removing, or updating key-value pairs in a map. The patching system ensures that changes are applied in a controlled manner, maintaining the integrity of the data structure.

#### Adding Key-Value Pairs

To add a new key-value pair to a map, you can use the following YAML syntax:

```yaml
map:
  +key: value
```

This will add the key `key` with the value `value` to the map.

#### Removing Key-Value Pairs

To remove a key-value pair from a map, you can use the following YAML syntax:

```yaml
map:
  key: null
```

This will remove the key `key` from the map.

#### Replacing Key-Value Pairs

To replace a key-value pair in a map, you can use the following YAML syntax:

```yaml
map:
  key: new_value
```

This will replace the value of the key `key` with `new_value`.

### Map Patch Behavior

Map patching in Dofigen YAML files allows you to modify key-value pairs in a map by adding, removing, or replacing them. This is particularly useful for managing configurations that involve dictionaries or objects, such as environment variables or settings.

### Map Patch Rules

- **Add Key-Value Pairs**: You can add new key-value pairs to a map by specifying the key and the value.
- **Remove Key-Value Pairs**: Key-value pairs can be removed from a map by specifying the key to be removed.
- **Replace Key-Value Pairs**: Existing key-value pairs can be replaced by specifying the key and the new value.

### Map Patch Examples

Here are some examples of how to patch maps in Dofigen YAML files:

```yaml
# Add a key-value pair to a map
map:
  +key: value

# Remove a key-value pair from a map
map:
  key: null

# Replace a key-value pair in a map
map:
  key: new_value
```

### Special Considerations

- **Nested Maps**: When patching nested maps, you can specify the path to the key-value pair you want to modify.
- **Conflicting Keys**: Dofigen allows conflicting keys in maps, but you should ensure that the configuration logic can handle them appropriately.



### Map Patching


## Array Patching

This represents the patching of arrays in Dofigen YAML files.

### Array Patch Examples

Here are some examples of how to patch arrays in Dofigen YAML files:

```yaml
# Add an element to an array
array:
  +: "new_element"

# Remove an element from an array
array:
  2: null

# Replace an element in an array
array:
  1: "updated_element"
```
