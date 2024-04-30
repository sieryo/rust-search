const input = document.getElementById("search");
const submitButton = document.getElementById("btn-submit");

submitButton.addEventListener("click", async (e) => {
    const query = input.value;
    const result = await requestSearch(query);

    let html = ``;

    result.forEach(item => {
        html += `<li>${item}</li>`;
    });

    document.getElementById('resultList').innerHTML = `<ul>${html}</ul>`;
});

const requestSearch = async (value) => {
  const res = await fetch("/api/search", {
    method: "POST",
    headers: {
      "Content-Type": "text/plain",
    },
    body: value,
  });
  return res.json();
};

input.addEventListener("click", () => {
  console.log("hello");
});

// fetch("/api/search", {
//     method: "POST",
//     headers: {
//         'Content-Type' : 'text/plain'
//     },
//     body: "Buffer.",
// }).then(response => console.log(response))
