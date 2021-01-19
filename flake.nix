{
  description = "WartID: the WartaPoirier authentication and authorization service";

  inputs.mozilla = { url = "github:mozilla/nixpkgs-mozilla"; flake = false; };
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, mozilla, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
        let
          rustOverlay = final: prev:
            let rustChannel = prev.rustChannelOf {
              rustToolchain = ./rust-toolchain;
              sha256 = "sha256-olL7mTcuWQ6d46WIBW8vK3DfsNGbgGBuWQnazALOkk8=";
            };
            in {
              inherit rustChannel;
              rustc = rustChannel.rust;
              cargo = rustChannel.rust;
            };
          nixpackages = nixpkgs.legacyPackages.${system};
        in 
        with import nixpkgs {
          inherit system;
          overlays = [
            (import "${mozilla}/rust-overlay.nix")
            rustOverlay
          ];
        };
    {

      packages.${system} = {
        # Discord bot
        wartid-server-discord-bot =
        pkgs.buildRustCrate {
          crateName = "wartid-server-discord-bot";
          pname = "wartid-server-discord-bot";
          version = "0.1.0";
          src = ./.;
          workspace_member = ./wartid-server-discord-bot;
          cargoSha256 = "sha256-olL7mTcuWQ6d46WIBW8vK3DfsNGbgGBuWQnazALOkk8=";
          meta = with lib; {
            description = "Discord bot WartID authentication";
            homepage = "https://github.com/WartaPoirier-corp/WartID/";
            license = licenses.agpl3;
          };
        };

        # Server
        wartid-server =
        pkgs.buildRustCrate {
          crateName = "wartid-server";
          pname = "wartid-server";
          version = "0.1.0";
          src = ./.;
          workspace_member = ./wartid-server;
          cargoSha256 = "sha256-olL7mTcuWQ6d46WIBW8vK3DfsNGbgGBuWQnazALOkk8=";
          meta = with lib; {
            description = "Discord bot WartID authentication";
            homepage = "https://github.com/WartaPoirier-corp/WartID/";
            license = licenses.agpl3;
          };
        };
      };

      defaultPackage.${system} = self.packages.${system}.wartid-server;
      devShell = nixpackages.mkShell {
        name = "wartid";
        buildInputs = with nixpackages; [ rustChannel.rust diesel-cli postgresql ];
        shellHook = ''
            export PGDATA=$PWD/postgres_data
            export PGHOST=$PWD/postgres
            export LOG_PATH=$PWD/postgres/LOG
            export PGDATABASE=wartid
            export DATABASE_URL="postgresql:///wartid?host=$PGHOST"
            if [ ! -d $PGHOST ]; then
              mkdir -p $PGHOST
            fi
            if [ ! -d $PGDATA ]; then
              echo 'Initializing postgresql database...'
              initdb $PGDATA --auth=trust >/dev/null
              pg_ctl start -l $LOG_PATH -o "-c listen_addresses= -k $PGHOST"
              createuser -d wartid
              createdb -O wartid wartid
            else
              pg_ctl start -l $LOG_PATH -o "-c listen_addresses= -k $PGHOST"
            fi
        '';
      };
    });
}
