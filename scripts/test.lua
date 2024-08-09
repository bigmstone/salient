local helper = require "helper"

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
                content = helper.build_prompt(helper.functions)
            },
            {
                role = "user",
                content = [[What's the weather in Ruston, Louisiana today?]]
            }
        }
    }

    local results = helper.eval(params)

    print(results[#results].content)
end
