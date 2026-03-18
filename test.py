import axios_python

def getReq(url):
    axios_python.get(url)
    print("Success")

getReq("https://www.google.com")