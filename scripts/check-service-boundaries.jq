.workspace_root as $root
| [
    .packages[]
    | select(.manifest_path | startswith($root + "/services/"))
    | . as $package
    | (.manifest_path | ltrimstr($root + "/services/") | split("/")[0]) as $service
    | .dependencies[]
    | select(.path != null and (.path | startswith($root + "/services/")))
    | select((.path | ltrimstr($root + "/services/") | split("/")[0]) != $service)
    | "\($package.name) must not depend on another service's Cargo package: \(.path)"
  ]
| if length == 0 then true else join("\n") | halt_error(1) end
