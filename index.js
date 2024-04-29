console.log("hellow rod");

fetch("/api/search", {
    method: "POST",
    headers: {
        'Content-Type' : 'text/plain'
    },
    body: "Buffer.",
}).then(response => console.log(response))