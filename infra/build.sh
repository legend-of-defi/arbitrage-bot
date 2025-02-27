#!/bin/bash

# Install required Ansible collections
ansible-galaxy collection install community.general
ansible-galaxy collection install ansible.posix
ansible-galaxy collection install community.docker
