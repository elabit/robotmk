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

TBD: automate this

To release `v1.0.4`:

* `export ROBOTMK_VERSION='1.0.4'`
* Merge features
    * Switch to `devel` branch 
    * merge all features, document in Changelog
* Changelog/Tag:
    * Document Changelog entries in `Unreleased`
    * Create a version entry: `chag update 1.0.4` (without "v")
    * edit version numbers in scripts (Version Bump)
    * Commit (make workdir clean)
    * Create annotated tag from Changelog entry: `chag tag --addv` (adds "v" in front of tag)
* If there are changes in the Github workflow, Push to `develop` (!)  
* Checkout `master`, merge `develop`:  `git merge develop --no-ff`
* Push to master: 
  * `git push origin master`
  * `git push origin v1.0.4`

* Create the Release on Github: 
