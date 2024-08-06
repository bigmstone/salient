#!/usr/bin/env lua
--[[
Verbosely fetches an HTTP resource
If a body is given, use a POST request

Usage: lua examples/simple_request.lua <URI> [<body>]
]]
local request = require "http.request"
local json = require "json"

local get_location = "http://api.openweathermap.org/geo/1.0/direct?q=%s&limit=1&appid=%s"
local get_weather = "https://api.openweathermap.org/data/3.0/onecall?lat=%s&lon=%s&appid=%s"

function get_uri(uri)
    local req_timeout = 10
    local req = request.new_from_uri(uri)

    local headers, stream = req:go(req_timeout)

    if headers == nil then
        io.stderr:write("HTTP request failed. Error: ", tostring(stream), "\n")
        os.exit(1)
    end

    local body, err = stream:get_body_as_string()
    if not body and err then
        io.stderr:write("Error reading response body: ", tostring(err), "\n")
        os.exit(1)
    end

    return body
end


function main()
    local location = assert(arg[1], "Location needed")

    local location_uri = string.format(get_location, location, os.getenv("OPEN_WEATHER_API_KEY"))
    local location_result = json.decode(get_uri(location_uri))

    local weather_uri = string.format(get_weather, location_result[1].lat, location_result[1].lon, os.getenv("OPEN_WEATHER_API_KEY"))
    local weather_result = get_uri(weather_uri)
    print(weather_result)
end

main()
