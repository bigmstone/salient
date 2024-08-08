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
