local json = require "json"

local prompt = [[You have access to the following functions:

Use the function 'get_weather' to 'retrieve the weather for a given location':
{
  "name": "get_weather",
  "description": "retrieve the weather for a given location",
  "parameters": {
    "type": "object",
    "properties": {
      "location": {
        "type": "string",
        "description": "Location in the format 'City, State'. Ensure you use the full state and not the short code."
      }
    }
  }
}

If you choose to call a function ONLY reply in the following format with no prefix or suffix:

<function=example_function_name>{"example_name": "example_value"}</function>

Reminder:
- Function calls MUST follow the specified format, start with <function= and end with </function>
- Required parameters MUST be specified
- Only call one function at a time
- Put the entire function call reply on one line
- If there is no function call available, answer the question like normal with your current knowledge and do not tell the user about function calls]]

function parse_function(message)
    local func_start, func_start_f = string.find(message, "<function=")
    if func_start ~= nil then
        local func_name_end, func_name_end_f = string.find(message, ">")
        if func_name_end ~= nil then
            local func_end_start, func_end_f = string.find(message, "</function>")
            if func_end_start ~= nil then
                local func_name = string.sub(message, func_start_f+1, func_name_end_f-1)
                local params = string.sub(message, func_name_end_f+1, func_end_start-1)
                print(string.format("Function: %s", func_name))
                print(string.format("Params: %s", params))

                local rendered_params = json_to_lua({ params = params })

                return {
                    func_name = func_name,
                    params = rendered_params
                }
            end
            error([[Malformed function definiton. Should follow `<function=function_name>{"param_name": "value"}` format.]])
        end
        error([[No function end found (</function>. Should follow `<function=function_name>{"param_name": "value"}` format.)]])
    end
end

function execute_function(message)
    local pass, func_info = pcall(parse_function, message)
    if pass == false then
        error(func_info)
    end
    if func_info ~= nil then
        if func_info.func_name == "get_weather" then
            print(string.format("Running get_weather for %s", func_info.params.location))
            return get_weather(func_info.params.location)
        end
    end
end


local base_location_uri = "https://api.openweathermap.org/geo/1.0/direct?q=%s&limit=1&appid=%s"
local base_weather_uri = "https://api.openweathermap.org/data/3.0/onecall?lat=%s&lon=%s&exclude=minutely,hourly&units=imperial&appid=%s"

function get_weather(location)
    location_uri = string.format(base_location_uri, percent_encode({input=location}).output, os.getenv("OPEN_WEATHER_API_KEY"))
    location_result = http_get({uri=location_uri})

    weather_uri = string.format(base_weather_uri, location_result[1].lat, location_result[1].lon, os.getenv("OPEN_WEATHER_API_KEY"))
    weather_result = http_get({uri=weather_uri})

    return weather_result
end

Eval = Eval or {}

function Eval.setup()
    print("Eval setup")
end

function Eval.execute()
    print("Eval execute")
    local params = {
        messages = {
            {
                role = "system",
                content = prompt
            },
            {
                role = "user",
                content = [[Hello, what is the forecast for Ruston, Louisiana?]]
            }
        }
    }

    local results = nil
    
    for i=1,10,1 do
        print(i)

        results = llm_eval(params)

        print("Running execute function")
        local pass, func_result = pcall(execute_function, results.content)

        if pass == false or func_result ~= nil then
            results.role = "function_call"
            table.insert(params["messages"], results)
            table.insert(params["messages"], {
                role = "function_result",
                content = json.encode(func_result)
            })

            print(json.encode(params))

            goto continue
        end

        break
        ::continue::
    end

    print(results)
    print("Eval done")
end
