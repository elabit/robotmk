# Consulting

Sometimes a user requires technical knowledge beyond what a documentation can provide.
Often, this gap is bridged by a technical consultant.
Here we describe functionality of `Robotmk`, which is directed at these technical consultants.

## Deployment

The `Robotmk suite scheduler` is deployed via the Checkmk Agent Bakery.
If you have a running Checkmk site, easily create a virtual machine with Vagrant to try the scheduler.
There is a Vagrantfile is located in the `vagrant` folder, which manages such a virtual machine.
Please refer to the Vagrant documentation on how to use it.

Currently, this file will install the Checkmk Windows Agent.

TODO: Robotmk suite scheduler should also be installed by the Vagrantfile.
