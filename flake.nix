{
  description = "WartID: the WartaPoirier authentication and authorization service";

  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.fenix.url = "github:figsoda/fenix?rev=f2ede107c26645dc1e96d3c0d9fdeefbdcc9eadb";
  inputs.fenix.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, fenix, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        rust = pkgs.makeRustPlatform {
          inherit (fenix.packages.${system}.minimal) cargo rustc;
        };
      in {
        packages = {
          # Discord bot
          wartid-server-discord-bot = rust.buildRustPackage {
            crateName = "wartid-server-discord-bot";
            pname = "wartid-server-discord-bot";
            version = "0.1.0";
            src = ./.;
            workspace_member = "wartid-server-discord-bot";
            cargoSha256 = "sha256-HgdJHE1OwKkJu2UC1zBOxuJcfCuUE2i//THo29PrK8c=";
            buildInputs = with pkgs; [ postgresql ];
            meta = with pkgs.lib; {
              description = "Discord bot WartID authentication";
              homepage = "https://github.com/WartaPoirier-corp/WartID/";
              license = licenses.agpl3;
            };
          };

          # Server
          wartid-server = rust.buildRustPackage {
            crateName = "wartid-server";
            pname = "wartid-server";
            version = "0.1.0";
            src = ./.;
            workspace_member = "wartid-server";
            cargoSha256 = "sha256-rIMx0x0i8Uj7AGgxAs3TMXGiQDoHQiA/PFhIq0rsDz4=";
            cargoBuildFlags = [ "--features" "discord_bot" ];
            buildInputs = with pkgs; [ postgresql ];
            meta = with pkgs.lib; {
              description = "Discord bot WartID authentication";
              homepage = "https://github.com/WartaPoirier-corp/WartID/";
              license = licenses.agpl3;
            };
          };
        };

        defaultPackage = self.packages.${system}.wartid-server;
        devShell = pkgs.mkShell {
          name = "wartid";
          buildInputs = with pkgs; [ fenix.packages.${system}.minimal.cargo fenix.packages.${system}.minimal.rustc diesel-cli postgresql ];
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
      }) // {
        nixosModule = { config, pkgs, lib, ... }:
          let
            cfg = config.services.wartid;
          in
          with lib;
          {
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
                  HTTP_BASE_URL = "http://localhost:${builtins.toString cfg.port}";
                };
                serviceConfig = {
                  ExecStart = "/${self.packages.${pkgs.system}.wartid-server}/bin/wartid-server";
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
                  DISCORD_TOKEN = cfg.discordToken;
                  DISCORD_ALLOWED_GUILD = concatStringsSep "," (builtins.map builtins.toString cfg.discordAllowedGuilds);
                };
                serviceConfig = {
                  ExecStart = "/${self.packages.${pkgs.system}.wartid-server-discord-bot}/bin/wartid-server-discord-bot";
                  Type = "simple";
                  User = "wartid";
                  Group = "wartid";
                };
              };
            };
          };
      };
}
