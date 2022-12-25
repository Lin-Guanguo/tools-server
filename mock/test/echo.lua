resp.status = 400
resp.header = {};
resp.header.mock_path = req.path
for key, value in pairs(req.query) do
    resp.header["query_" .. key] = value
end
for key, value in pairs(req.header) do
    resp.header["header_" .. key] = value
end
resp.body = req.body;
