use std::{env, fs};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("cargo:rerun-if-changed=api-definitions/proto/");

	let base_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

	tonic_build::configure()
		.out_dir(&base_dir)
		.compile(
			&vec![
				"api-definitions/proto/common.proto",
			],
			&vec!["api-definitions/proto"]
		)?;

	for category in vec!["address", "transaction"] {
		let dir = base_dir.join("transaction");
		fs::create_dir_all(&dir)?;
		tonic_build::configure()
			.build_client(true)
			.build_server(true)
			.out_dir(dir)
			.server_mod_attribute("emerald", format!("#[cfg(feature = \"server-{}\")]", category).as_str())
			.client_mod_attribute("emerald", format!("#[cfg(feature = \"client-{}\")]", category).as_str())
			.compile(
				&vec![
					format!("api-definitions/proto/{}.proto", category),
					format!("api-definitions/proto/{}.message.proto", category),
				],
				&vec!["api-definitions/proto"]
			)?;
	}


	for category in vec!["auth", "blockchain", "market", "monitoring"] {

		let dir = base_dir.join(category);
		fs::create_dir_all(&dir)?;

		tonic_build::configure()
			.build_client(true)
			.build_server(true)
			.out_dir(dir)
			.server_mod_attribute("emerald", format!("#[cfg(feature = \"server-{}\")]", category).as_str())
			.client_mod_attribute("emerald", format!("#[cfg(feature = \"client-{}\")]", category).as_str())
			.compile(
				&vec![
					format!("api-definitions/proto/{}.proto", category),
				],
				&vec!["api-definitions/proto"]
			)?;
	}

    Ok(())
}
