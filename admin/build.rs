use std::fs;
use std::path::Path;

fn main() {
    let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/dist");
    let index = dist.join("index.html");
    if !index.exists() {
        fs::create_dir_all(&dist).expect("create web/dist");
        fs::write(
            &index,
            r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>NexusGate Admin</title></head>
<body>
  <p>Frontend assets missing. Run <code>cd web &amp;&amp; npm ci &amp;&amp; npm run build</code> then rebuild p2p-admin.</p>
</body>
</html>
"#,
        )
        .expect("write placeholder index.html");
        println!("cargo:warning=web/dist missing; embedded placeholder index.html");
    }
    println!("cargo:rerun-if-changed=../web/dist");
}
