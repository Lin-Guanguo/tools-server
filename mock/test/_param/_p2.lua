-- echo path param

resp.header = {};
resp.header.mock_path = path
for key, value in pairs(req.path_param) do
    resp.header["path_" .. key] = value
end
