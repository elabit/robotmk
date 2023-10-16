# Consulting

Sometimes a user requires technical knowledge beyond what a documentation can provide.
Often, this gap is bridged by a technical consultant.
Here we describe functionality of `Robotmk`, which is directed at these technical consultants.

## Deployment

The `Robotmk suite scheduler` is deployed via the Checkmk Agent Bakery.
If you have a running Checkmk site, you can create a virtual machine with Vagrant to try the scheduler.
The Vagrantfile is located in the `vagrant` folder.
Please refer to the Vagrant documentation on how to use it.

In order for the Vagrantfile to install the Checkmk Windows Agent, there are two prerequisites.
First, you will need to install the requirements described by `vagrant/setup.md`.
Secondly, you will need a running Checkmk site.
You can specify the site by using the file `vagrant/default_settings.yaml`.
Since `default_settings.yaml` is tracked by `git`, you should make a copy:
```
cd vagrant
cp default_settings.yaml custom_settings.yaml
```
Once you have adapted `custom_settings.yaml`, you can run the provisioning step
```
ROBOTMK_VAGRANT_CONFIG=custom_settings.yaml vagrant provision
```

TODO: Robotmk bakery plugin should also be configured by the Vagrantfile.
