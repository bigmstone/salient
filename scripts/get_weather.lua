local get_weather = {
    _version = "0.0.1",
    base_location_uri = "https://api.openweathermap.org/geo/1.0/direct?q=%s&limit=1&appid=%s",
    base_weather_uri = "https://api.openweathermap.org/data/3.0/onecall?lat=%s&lon=%s&exclude=minutely,hourly&units=imperial&appid=%s",
}


function get_weather.exec(location)
    location_uri = string.format(get_weather.base_location_uri, percent_encode({input=location}).output, os.getenv("OPEN_WEATHER_API_KEY"))
    location_result = http_get({uri=location_uri})

    weather_uri = string.format(get_weather.base_weather_uri, location_result[1].lat, location_result[1].lon, os.getenv("OPEN_WEATHER_API_KEY"))
    weather_result = http_get({uri=weather_uri})

    return weather_result
end

function get_weather.register()
    return {
        name = "get_weather",
        description = "Retrieve the weather for a given location",
        params = {
            location = {
                description = "Location in the format 'City, State'. Ensure you use the full state and not the short code.",
                dtype = "string",
                examples = {"Ruston, Louisiana", "Fort Collins, Colorado", "Orange Beach, Alabama"}
            }
        }
    }
end

return get_weather
