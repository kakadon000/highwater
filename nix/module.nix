{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.highwater;
in
{
  options.services.highwater = {
    enable = lib.mkEnableOption "the Highwater user daemon";
    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.highwater;
      defaultText = lib.literalExpression "inputs.highwater.packages.\${pkgs.system}.highwater";
      description = "The Highwater package to run.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
    services.dbus.packages = [ cfg.package ];
    systemd.user.services.highwaterd = {
      description = "Highwater quota daemon";
      after = [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      wantedBy = [ ];
      serviceConfig = {
        Type = "dbus";
        BusName = "io.github.kakadon000.Highwater.Daemon";
        ExecStart = "${cfg.package}/bin/highwaterd";
        Restart = "on-failure";
        RestartSec = 5;
        NoNewPrivileges = true;
      };
    };
  };
}
