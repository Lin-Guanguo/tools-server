-- echo path param

resp_header = {};
resp_header.mock_path = path
for key, value in pairs(path_param) do
    resp_header["path_" .. key] = value
end
