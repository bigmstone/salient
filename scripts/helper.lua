local json = require "json"
local get_weather = require "get_weather"

local helper = {
    _version = "0.0.1",
    functions = {
        get_weather = get_weather.register(),
    }
}



function helper.build_prompt(functions)
    local function_text = ""

    for k, v in pairs(functions) do
        local param_text = nil

        for k, v in pairs(v.params) do
            local param = string.format([[
        "%s": {
            "type": "%s",
            "description": "%s"
        }]], k, v.dtype, v.description)
            if param_text ~= nil then
                param_text = param_text .. ",\n" .. param
            else 
                param_text = param
            end
        end

        param_text = "\n" .. param_text .. "\n    "

        function_text = function_text .. string.format([[
Use the function '%s' to '%s':
{
    "name": "%s",
    "description": "%s",
    "parameters": {%s}
}
        
        ]], v.name, v.description, v.name, v.description, param_text)
    end

    local prompt = string.format([[You have access to the following functions:

%s

If you choose to call a function ONLY reply in the following format with no prefix or suffix:

<function=example_function_name>{"example_name": "example_value"}</function>

Reminder:
- Function calls MUST follow the specified format, start with <function= and end with </function>
- Required parameters MUST be specified
- Only call one function at a time
- Put the entire function call reply on one line
- If there is no function call available, answer the question like normal with your current knowledge and do not tell the user about function calls]],
    function_text)

    return prompt
end

function helper.parse_function(message)
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
            error([[Malformed function definiton. Should follow `<function=function_name>{"param_name": "value"}</function>` format.]])
        end
        error([[No function end found (</function>. Should follow `<function=function_name>{"param_name": "value"}</function>` format.)]])
    end
end

function helper.execute_function(message)
    local pass, func_info = pcall(helper.parse_function, message)
    if pass == false then
        error(func_info)
    end
    if func_info ~= nil then
        if func_info.func_name == "get_weather" then
            print(string.format("Running get_weather for %s", func_info.params.location))
            return get_weather.exec(func_info.params.location)
        end
    end
end


function helper.eval(params)
    local results = nil
    
    for i=1,10,1 do
        print(i)

        results = llm_eval(params)

        print("Running execute function")
        local pass, func_result = pcall(helper.execute_function, results.content)

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

    table.insert(params["messages"], results)

    return params["messages"]
end

return helper
