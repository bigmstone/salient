# Salient

Experimental LLM agent targeted mostly at myself. Should feel at home if you're a software
developer. Not exactly reliable. Focusing on functionality and not sanding the soft edges for now.

## Config

### LLM Choice

You can use HF models directly (will be downloaded on first run) by adding the repo and model (GGUF
only).

```
[model.HuggingFace]
repo = "QuantFactory/dolphin-2.9-llama3-8b-GGUF"
model = "dolphin-2.9-llama3-8b.Q8_0.gguf"
```

Alternatively you can point to a GGUF model on disk using path.

```
[model.Local]
path = "/path/to/model.gguf"
```

### Scripts and Tasks

Scripts come in the form of Lua scripts. They can be placed anywhere. `LUA_PATH` is automatically
updated to allow you to `require` additional Lua scripts from the same folder. Tasks take the shape
of specific named tables in the Lua script. As an example, take the following Lua script.

```
Test = Test or {}

function Test.setup()
    print("Test setup")
end

function Test.execute()
    print("Test execute")
    local params = {
        messages = {
            {
                role = "system",
                content = [[You are a helpful AI assistant.]]
            },
            {
                role = "user",
                content = [[Plan a 5 day 4 night trip to Paris, France. Include places to eat and things to do.]]
            }
        }
    }

    results = llm_eval(params)

    print(results.content)
end
```

Test is the name of the task. The `setup` method is run immediatly upon loading the script, and the
`execute` method is executed on the cron defined schedule. The script, task(s) name(s), and cron
schedule(s) are configured in the following way.

```
[[scripts]]
path = "./scripts/test.Lua"

[[scripts.tasks]]
cron = "0 * * * * * *"
name = "Test"

[[scripts]]
path = "./scripts/test2.Lua"

[[scripts.tasks]]
cron = "0 * * * * * *"
name = "TestTwo"
```

You can add as many scripts as you like.

## Exposed Functions

There are several functions exposed to the Lua scripts from the Rust runtime to the Lua runtime. All
functions are currently defined in `main.rs` and relatively easy to extend if you desire. All
functions also have access to a configureale `Scope` for access to shared resources.

### llm_eval

Executes the LLM with the provided context.

#### Param(s)

- `messages` - Array of objects. Each object has a `role` element and a `content` string
  - `role` - String that should contain `system` or `user` to denote the author of the content
  - `content` - String containing the message to the LLM

#### Return Value(s)

- `message` - Object containing `role` and `content` arguments filled in from the LLM

### http_get

Provides basic HTTP/HTTPS get for provided URI.

#### Param(s)

- `uri` - String of the full address to perfrom the HTTP/HTTPS GET against.

#### Return Value(s)

- Lua table of the resutls of the HTTP/HTTPS GET.

### percent_encode

Encodes the provided String with URI percent encoding.

#### Param(s)

- `input` - String to percent encode

#### Return Value(s)

- `output` - Percent encoded String

### json_to_lua

Renders a valid JSON string into a Lua table.

#### Param(s)

- `params` - String containing the valid JSON to seralize into a Lua table.

#### Return Value(s)

- Lua table of the contents of the JSON String provided in `params`.
