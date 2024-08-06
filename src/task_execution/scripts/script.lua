local request = require "http.request"
local json = require "json"

function get_uri(uri)
    local req_timeout = 10
    local req = request.new_from_uri(uri)

    local headers, stream = req:go(req_timeout)

    if headers == nil then
        error(string.format("HTTP request failed. Error: %s", tostring(stream)))
    end

    local body, err = stream:get_body_as_string()
    if not body and err then
        error(string.format("Error reading response body: %s", tostring(err)))
    end

    return body
end

function get_weather(location)
    local location_uri = string.format(get_location, location, os.getenv("OPEN_WEATHER_API_KEY"))
    local location_result = json.decode(get_uri(location_uri))

    local weather_uri = string.format(get_weather, location_result[1].lat, location_result[1].lon, os.getenv("OPEN_WEATHER_API_KEY"))
    local weather_result = get_uri(weather_uri)
    print(weather_result)
end

local get_location = "http://api.openweathermap.org/geo/1.0/direct?q=%s&limit=1&appid=%s"
local get_weather = "https://api.openweathermap.org/data/3.0/onecall?lat=%s&lon=%s&appid=%s"

local prompt = [[You are a helpful assistant that will respond with both messages or function calls, as needed. You have access to the following functions:
{
  "functions": [
    {
      "name": "get_weather",
      "description": "Retrieves the weather for a given location.",
      "parameters": {
        "type": "object",
        "properties": {
          "location": {
            "type": "string",
            "description": "Location in the format 'City, State'. Ensure you use the full state and not the short code."
          }
        },
        "required": ["location"]
      }
    }
  ]
}

You can respond to user messages with either a single message or one, and only one, function calls.

To respond with a message, simply begin the message. To respond with one or more function calls, begin the message with 'functions.<function_name>:', use the following format:

functions.get_weather:
{ "location": "New York, New York" }


If you chose to execute a function only include the above format. No further message is necessary. You will be given the results of the function call in a follow-up message.
]]

function execute_function(message)
    
end

Eval = Eval or {}

function Eval.setup()
    print("Eval setup")
end

function Eval.execute()
    print("Eval execute")
    params = {
        messages = {
            {
                role = "System",
                content = prompt
            },
            {
                role = "User",
                content = [[Hello, what is the weather in Chicago, Illinois?]]
            }
        }
    }
    results = llm_eval(params)
    print(results.content)
end
