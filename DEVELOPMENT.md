# Robotmk Toolstack

I thought it would be a good idea to document all the tools and tricks I have used in this project to help others - and lastly myself.

## Version Control and Releasing

### Changelog

Robotmk's [CHANGELOG.md](CHANGELOG.md) is based on [](https://keepachangelog.com/).

### Chag

Robotmk uses [chag](https://raw.githubusercontent.com/mtdowling/chag/master/install.sh) to keep annotated tags and the CHANGELOG in sync. 

All unreleased work is documented under the `H2` "Unreleased": 

    ## Unreleased

    This will be the release title 

    
  * Show entries of a special release: `chag contents --tag v1.0.2`
  * Create a Changelog entry for the `Unreleased` section: `chag update 1.0.4`
  * Create an annotated tag from the 


## Release Workflow 

The release workflow of Robotmk is divided into the following steps: 

* Make sure that the `develop` branch is clean (=everything is stashed/committed)
* Execute `./release.sh release 1.2.0`, which 
  * executes `chag update` => converting unreleased entries in `CHANGELOG` to the new version
  * replaces version number variables in Robotmk script files
  * commits this change as version bump 
  * merges `develop` into `master`
  * executes `chag tag --addv` => adds an annotated tag from the Changelog
  * pushes to `master`
