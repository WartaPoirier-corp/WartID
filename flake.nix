{
  description = "WartID: the WartaPoirier authentication and authorization service";

  inputs.mozilla = { url = "github:mozilla/nixpkgs-mozilla"; flake = false; };

  outputs = { self, nixpkgs, mozilla }:
    let
      rustOverlay = final: prev:
        let rustChannel = prev.rustChannelOf {
          rustToolchain = ./rust-toolchain;
          sha256 = "sha256-uoGBMgGmIPj4E+jCY8XH41Ia8NbaRjDC3KioiaCA/M8=";
        };
        in {
          inherit rustChannel;
          rustc = rustChannel.rust;
          cargo = rustChannel.rust;
        };
    in 
    with import nixpkgs {
      system = "x86_64-linux"; # TODO: build for other systems too
      overlays = [
        (import "${mozilla}/rust-overlay.nix")
        rustOverlay
      ];
    };
    {

      # Discord bot
      packages.x86_64-linux.wartid-server-discord-bot =
        pkgs.buildRustCrate {
          crateName = "wartid-server-discord-bot";
          pname = "wartid-server-discord-bot";
          version = "0.1.0";
          src = ./.;
          workspace_member = ./wartid-server-discord-bot;
          cargoSha256 = lib.fakeSha256;
          meta = with lib; {
            description = "Discord bot WartID authentication";
            homepage = "https://github.com/WartaPoirier-corp/WartID/";
            license = licenses.agpl3;
          };
        };

      defaultPackage.x86_64-linux = self.packages.x86_64-linux.wartid-server-discord-bot;
      devShell.x86_64-linux = pkgs.mkShell {
        name = "wartid";
        buildInputs = with pkgs; [ rustChannel.rust diesel-cli postgresql ];
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
    };
}
