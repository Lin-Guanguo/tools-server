-- const LUA_ARGS_PATH: &str = "path";
-- const LUA_ARGS_PATH_PARAM: &str = "path_param";
-- const LUA_ARGS_QUERY: &str = "query";
-- const LUA_ARGS_HEADER: &str = "header";
-- const LUA_ARGS_BODY: &str = "body";
-- const LUA_RESP_STATUS: &str = "resp_status";
-- const LUA_RESP_HEADER: &str = "resp_header";
-- const LUA_RESP_BODY: &str = "resp_body";

resp_status = 400
resp_header = {};
resp_header.mock_path = path
for key, value in pairs(query) do
    resp_header["query_" .. key] = value
end
for key, value in pairs(header) do
    resp_header["header_" .. key] = value
end
resp_body = body;
