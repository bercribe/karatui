{
  pkgs,
  config,
  lib,
  ...
}: let
  cfg = config.programs.karatui;
in {
  options = with lib;
  with types; {
    programs.karatui = {
      enable = mkEnableOption "karatui";
      settings = {
        url = mkOption {
          type = str;
          example = "https://try.karakeep.app";
          description = "URL of your karakeep instance";
        };
        list_id = mkOption {
          type = str;
          example = "xxxxxxxxxxxxxxxxxxxxxxxx";
          description = "ID of the list to open";
        };
        api_key_path = mkOption {
          type = str;
          example = "/path/to/key";
          description = "Path to the file containing your karakeep API key";
        };
      };
    };
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."karatui/karatui.toml" = {
      source = (pkgs.formats.toml {}).generate "karatui.toml" (with cfg.settings; {
        inherit url list_id api_key_path;
      });
    };
  };
}
