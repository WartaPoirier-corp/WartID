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
      packages = {
        # Discord bot
        wartid-server-discord-bot = pkgs.buildRustCrate {
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
        wartid-server = pkgs.buildRustCrate {
          crateName = "wartid-server";
          pname = "wartid-server";
          version = "0.1.0";
          src = ./.;
          features = [ "discord-bot" ];
          workspace_member = ./wartid-server;
          cargoSha256 = "sha256-olL7mTcuWQ6d46WIBW8vK3DfsNGbgGBuWQnazALOkk8=";
          meta = with lib; {
            description = "Discord bot WartID authentication";
            homepage = "https://github.com/WartaPoirier-corp/WartID/";
            license = licenses.agpl3;
          };
        };
      };

      defaultPackage = self.packages.${system}.wartid-server;
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
    }) // (with nixpkgs.lib; {
      nixosModule = { config, pkgs }:
        let
          cfg = config.services.wartid;
        in {
          options.services.wartid = {
            enable = mkEnableOption "WartID server";
            enableDiscordBot = mkEnableOption "WartID Discord bot";
            db = {
              autoCreate = mkOption {
                type = types.bool;
                default = true;
                description = ''
                  true if you want NixOS to handle the creation of the database for you, false if you want to do it manually.
                  In either case, you will need to enable services.postgres
                '';
              };
              user = mkOption {
                type = types.string;
                default = "wartid";
                description = "The database user";
              };
              password = mkOption {
                type = types.string;
                description = "The database password";
              };
              name = mkOption {
                type = types.string;
                default = "wartid";
                description = "The database name";
              };
            };
            discordToken = mkOption {
              type = types.string;
              description = "The Discord token for the bot.";
            };
            discordAllowedGuilds = mkOption {
              type = types.listOf types.int;
              description = "Snowflake IDs of Guilds the bot accepts people from.";
            };
            port = mkOption {
              type = types.int;
              default = 7878;
              description = "HTTP listen port";
            };
          };

          config = mkIf cfg.enable {
            systemd.tmpfiles.rules = [
              "d /tmp/wartid/ - wartid wartid - -"
            ];
            users.users.wartid = {
              group = "wartid";
            };
            systemd.services.wartid-server = {
              description = "WartID server";
              wantedBy = [ "multi-user.target" ];
              after = [ "network.target" ];
              environment = {
                DISCORD_KEY_FILE = "/tmp/wartid/discord_jwt.key";
                DATABASE_URL = "postgres://${cfg.db.user}:${cfg.db.password}@localhost/${cfg.db.name}";
                HTTP_BASE_URL = "http://localhost:${cfg.port}";
              };
              serviceConfig = {
                ExecStart = "/${pkgs.wartid-server}";
                Type = "simple";
                User = "wartid";
                Group = "wartid";
              };
            };
            systemd.services.wartid-server-discord-bot = mkIf cfg.enableDiscordBot {
              description = "WartID server: Discord bot";
              wantedBy = [ "multi-user.target" ];
              after = [ "network.target" ];
              environment = {
                DISCORD_KEY_FILE = "/tmp/wartid/discord_jwt.key";
              };
              serviceConfig = {
                ExecStart = "/${pkgs.wartid-server-discord-bot}";
                Type = "simple";
                User = "wartid";
                Group = "wartid";
              };
            };
          };
      };
    });
}
