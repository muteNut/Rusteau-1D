(function () {
  var q = (new URLSearchParams(window.location.search).get("q") || "").trim();
  var box = document.querySelector("form.search input");
  if (box) { box.value = q; }
  var out = document.getElementById("results");
  var summary = document.getElementById("summary");
  var terms = q.toLowerCase().split(/\s+/).filter(function (t) { return t.length > 0; });
  if (terms.length === 0) { summary.textContent = "Enter words to search for."; return; }
  var hits = [];
  window.HELP_INDEX.forEach(function (t) {
    var text = t.text.toLowerCase(), title = t.title.toLowerCase(), score = 0, ok = true;
    terms.forEach(function (w) {
      var n = text.split(w).length - 1;
      if (n === 0) { ok = false; }
      score += n + (title.indexOf(w) >= 0 ? 20 : 0);
    });
    if (ok) { hits.push({ t: t, score: score }); }
  });
  hits.sort(function (a, b) { return b.score - a.score; });
  summary.textContent = hits.length + (hits.length === 1 ? " topic" : " topics") + " contain " + terms.map(function (w) { return "“" + w + "”"; }).join(" and ") + ".";
  hits.forEach(function (h) {
    var li = document.createElement("li");
    var a = document.createElement("a");
    a.href = h.t.id + ".html";
    a.textContent = h.t.title;
    li.appendChild(a);
    var text = h.t.text, i = text.toLowerCase().indexOf(terms[0]);
    var s = Math.max(0, i - 80), snip = text.substring(s, i + 120).replace(/\s+/g, " ");
    var p = document.createElement("p");
    p.textContent = (s > 0 ? "…" : "") + snip + "…";
    li.appendChild(p);
    out.appendChild(li);
  });
})();
