# This setup method is taken mostly from CMS (https://github.com/cms-dev/cms)
# https://github.com/cms-dev/cms/blob/master/LICENSE.txt


import subprocess
import os
import pwd
import shutil

USR_ROOT = os.path.join("/", "usr", "local")

def copyfile(src, dest, owner, perm, group=None):
    """Copy the file src to dest, and assign owner and permissions.

    src (string): the complete path of the source file.
    dest (string): the complete path of the destination file (i.e.,
                   not the destination directory).
    owner (as given by pwd.getpwnam): the owner we want for dest.
    perm (integer): the permission for dest (example: 0o660).
    group (as given by grp.getgrnam): the group we want for dest; if
                                      not specified, use owner's
                                      group.

    """
    shutil.copy(src, dest)
    owner_id = owner.pw_uid
    if group is not None:
        group_id = group.gr_gid
    else:
        group_id = owner.pw_gid
    os.chown(dest, owner_id, group_id)
    os.chmod(dest, perm)


def makedir(dir_path, owner=None, perm=None):
    """Create a directory with given owner and permission.

    dir_path (string): the new directory to create.
    owner (as given by pwd.getpwnam): the owner we want for dest.
    perm (integer): the permission for dest (example: 0o660).

    """
    if not os.path.exists(dir_path):
        os.makedirs(dir_path)
    if perm is not None:
        os.chmod(dir_path, perm)
    if owner is not None:
        os.chown(dir_path, owner.pw_uid, owner.pw_gid)

def build_isolate():
    print("Building Isolate")

    subprocess.check_call(["make", "-C", "isolate", "isolate"])

def setup_isolate():
    print("Setting up isolate")

    root = pwd.getpwnam("root")

    print("===== Copying isolate to /usr/local/bin/")
    makedir(os.path.join(USR_ROOT, "bin"), root, 0o755)
    copyfile(os.path.join(".", "isolate", "isolate"),
             os.path.join(USR_ROOT, "bin", "isolate"),
             root, 0o4750)

    print("===== Copying isolate config to /usr/local/etc/")
    makedir(os.path.join(USR_ROOT, "etc"), root, 0o755)
    copyfile(os.path.join(".", "isolate", "default.cf"),
             os.path.join(USR_ROOT, "etc", "isolate"),
             root, 0o640)

if __name__ == "__main__":
    build_isolate()
    setup_isolate()