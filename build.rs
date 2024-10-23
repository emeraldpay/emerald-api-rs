use std::{env, fs};
use std::path::PathBuf;
use tonic_build::Builder;

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

	for category in vec!["address", "token", "transaction"] {
		let dir = base_dir.join(category);
		fs::create_dir_all(&dir)?;

		let builder = tonic_build::configure()
			.build_client(true)
			.build_server(true)
			.out_dir(dir)
			.server_mod_attribute("emerald", format!("#[cfg(feature = \"server-{}\")]", category).as_str())
			.client_mod_attribute("emerald", format!("#[cfg(feature = \"client-{}\")]", category).as_str());
		let builder = link_common(builder);
		builder.compile(
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

		let builder = tonic_build::configure()
			.build_client(true)
			.build_server(true)
			.out_dir(dir)
			.server_mod_attribute("emerald", format!("#[cfg(feature = \"server-{}\")]", category).as_str())
			.client_mod_attribute("emerald", format!("#[cfg(feature = \"client-{}\")]", category).as_str());
		let builder = link_common(builder);
		builder.compile(
				&vec![
					format!("api-definitions/proto/{}.proto", category),
				],
				&vec!["api-definitions/proto"]
			)?;
	}

    Ok(())
}

///
/// Links the shared common types.
/// I.e., tell Protoc to use the specified Rust types, because otherwise it generates those types in each module.
fn link_common(builder: Builder) -> Builder {
	let ns = "crate::proto::common";
	builder
        .extern_path(".emerald.Chain", format!("{}::Chain", ns).as_str())
        .extern_path(".emerald.SingleAddress", format!("{}::SingleAddress", ns).as_str())
        .extern_path(".emerald.XpubAddress", format!("{}::XpubAddress", ns).as_str())
        .extern_path(".emerald.MultiAddress", format!("{}::MultiAddress", ns).as_str())
        .extern_path(".emerald.ReferenceAddress", format!("{}::ReferenceAddress", ns).as_str())
        .extern_path(".emerald.AnyAddress", format!("{}::AnyAddress", ns).as_str())
        .extern_path(".emerald.Asset", format!("{}::Asset", ns).as_str())
        .extern_path(".emerald.Erc20Asset", format!("{}::Erc20Asset", ns).as_str())
        .extern_path(".emerald.BlockInfo", format!("{}::BlockInfo", ns).as_str())
        .extern_path(".emerald.ChainRef", format!("{}::ChainRef", ns).as_str())
}