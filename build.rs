use std::{env, fs};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let base_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

	tonic_build::configure()
		.out_dir(&base_dir)
		.compile(
			&vec![
				"api-definitions/proto/common.proto",
			],
			&vec!["api-definitions/proto"]
		)?;

	let dir = base_dir.join("transaction");
	fs::create_dir_all(&dir)?;
	tonic_build::configure()
		.build_client(true)
		.build_server(true)
		.out_dir(dir)
		.server_mod_attribute("emerald", "#[cfg(feature = \"server-transaction\")]")
		.client_mod_attribute("emerald", "#[cfg(feature = \"client-transaction\")]")
		.compile(
			&vec![
				"api-definitions/proto/transaction.message.proto",
				"api-definitions/proto/transaction.proto",
			],
			&vec!["api-definitions/proto"]
		)?;


	let categories = vec!["auth", "blockchain", "market", "monitoring"];
	for category in categories {

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
